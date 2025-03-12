use crate::db::model::{
    ContractFactoryDbObject, FancyDbObj, FancyProviderDbObj, JobDbObj, MinerDbObj,
    PublicKeyBaseDbObject,
};
use crate::types::DbAddress;
use chrono::{DateTime, Utc};
use sqlx::{Executor, FromRow, Sqlite, SqlitePool};
use crate::db::executor::DbExecutor;

pub async fn insert_fancy_obj<'e, 'c : 'e>(
    exec: &'e mut DbExecutor<'c>,
    fancy_data: FancyDbObj,
) -> Result<FancyDbObj, sqlx::Error>
{
    let res = exec.exec().fetch_one(sqlx::query_as::<_, FancyDbObj>(
        r"INSERT INTO fancy
(address, salt, factory, created, score, job, owner, price, category, public_key_base)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *;
",
    )
    .bind(fancy_data.address)
    .bind(&fancy_data.salt)
    .bind(fancy_data.factory)
    .bind(fancy_data.created)
    .bind(fancy_data.score)
    .bind(fancy_data.job)
    .bind(&fancy_data.owner)
    .bind(fancy_data.price)
    .bind(&fancy_data.category)
    .bind(&fancy_data.public_key_base)
   )
    .await?;
    Ok(FancyDbObj::from_row(&res)?)
}

pub async fn fancy_list_all(
    conn: &SqlitePool,
    since: Option<DateTime<Utc>>,
) -> Result<Vec<FancyDbObj>, sqlx::Error> {
    let res = sqlx::query_as::<_, FancyDbObj>(r"SELECT * FROM fancy WHERE created > $1;")
        .bind(
            since
                .map(|s| s.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or("2000-01-01 00:00:00".to_string()),
        )
        .fetch_all(conn)
        .await?;
    Ok(res)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FancyOrderBy {
    Score,
    Created,
}

pub enum ReservedStatus {
    All,
    Reserved,
    NotReserved,
    User(String),
}
pub enum PublicKeyFilter {
    All,
    Selected(String),
    OnlyNull,
}

pub async fn get_public_key_list<'c>(
    conn: &mut DbExecutor<'c>,
    user_id: Option<String>,
) -> Result<Vec<PublicKeyBaseDbObject>, sqlx::Error>

{
    let where_clause = match user_id {
        Some(uid) => format!("WHERE user_id = '{}' OR user_id is NULL", uid),
        None => "".to_string(),
    };

    let res = sqlx::query_as::<_, PublicKeyBaseDbObject>(&format!(
        r"SELECT * FROM public_key_base {where_clause};"
    ))
    .fetch_all(conn.exec())
    .await?;
    Ok(res)
}

//@todo add public key base
#[allow(dead_code)]
pub async fn get_or_insert_public_key<'c>(
    conn: &mut DbExecutor<'c>,
    public_key_base: &str,
) -> Result<PublicKeyBaseDbObject, sqlx::Error>
{
    //select first
    let res = sqlx::query_as::<_, PublicKeyBaseDbObject>(
        r"SELECT * FROM public_key_base WHERE hex = $1;",
    )
    .bind(public_key_base)
    .fetch_optional(conn.exec())
    .await?;

    if let Some(pk) = res {
        Ok(pk)
    } else {
        let res = sqlx::query_as::<_, PublicKeyBaseDbObject>(
            r"INSERT INTO public_key_base (id, hex, added) VALUES ($1, $2, $3) RETURNING *;",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(public_key_base)
        .bind(Utc::now().naive_utc())
        .fetch_one(conn.exec())
        .await?;
        Ok(res)
    }
}

pub async fn get_or_insert_factory<'c>(
    conn: &mut DbExecutor<'c>,
    factory_address: DbAddress,
) -> Result<ContractFactoryDbObject, sqlx::Error>
{
    //select first
    let res = sqlx::query_as::<_, ContractFactoryDbObject>(
        r"SELECT * FROM contract_factory WHERE address = $1;",
    )
    .bind(factory_address)
    .fetch_optional(conn.exec())
    .await?;

    if let Some(pk) = res {
        Ok(pk)
    } else {
        let res = sqlx::query_as::<_, ContractFactoryDbObject>(
            r"INSERT INTO contract_factory (id, address, added) VALUES ($1, $2, $3) RETURNING *;",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(factory_address)
        .bind(Utc::now().naive_utc())
        .fetch_one(conn.exec())
        .await?;
        Ok(res)
    }
}

pub async fn fancy_list<'c>(
    conn: &mut DbExecutor<'c>,
    category: Option<String>,
    order_by: FancyOrderBy,
    reserved: ReservedStatus,
    since: Option<DateTime<Utc>>,
    public_key_base: PublicKeyFilter,
    limit: i64,
) -> Result<Vec<FancyProviderDbObj>, sqlx::Error>

{
    let order_by = match order_by {
        FancyOrderBy::Score => "score",
        FancyOrderBy::Created => "created",
    };

    let owner_condition = match reserved {
        ReservedStatus::All => "".to_string(),
        ReservedStatus::Reserved => "f.owner is NOT NULL".to_string(),
        ReservedStatus::NotReserved => "f.owner is NULL".to_string(),
        ReservedStatus::User(user) => format!("f.owner = '{}'", user).to_string(),
    };

    let public_key_base_condition = match public_key_base {
        PublicKeyFilter::All => "".to_string(),
        PublicKeyFilter::Selected(pk) => format!("f.public_key_base = '{}'", pk),
        PublicKeyFilter::OnlyNull => "f.public_key_base is NULL".to_string(),
    };

    let created_condition = match since {
        Some(since) => format!("f.created > '{}'", since.format("%Y-%m-%d %H:%M:%S")),
        None => "".to_string(),
    };

    let category_condition = match category {
        Some(cat) => format!("f.category = '{}'", cat),
        None => "".to_string(),
    };

    let where_clause = [
        owner_condition,
        public_key_base_condition,
        category_condition,
        created_condition,
    ]
    .into_iter()
    .filter(|x| !x.is_empty())
    .collect::<Vec<_>>()
    .join(" AND ");

    let limit_clause = if limit > 0 {
        format!("LIMIT {}", limit)
    } else {
        "".to_string()
    };

    let order_by_clause = format!("{} DESC", order_by);

    let res = sqlx::query_as::<_, FancyProviderDbObj>(
        format!(
            r"SELECT f.*, mi.prov_name, mi.prov_node_id, mi.prov_reward_addr
            FROM fancy as f LEFT JOIN job_info as ji ON f.job=ji.uid LEFT JOIN miner_info as mi ON mi.uid=ji.miner
            WHERE {where_clause}
            ORDER BY {order_by_clause}
            {limit_clause}"
        )
        .as_str(),
    )
    .fetch_all(conn.exec())
    .await?;
    Ok(res)
}

pub async fn fancy_get_by_address<'c>(
    conn: &mut DbExecutor<'c>,
    address: DbAddress,
) -> Result<Option<FancyDbObj>, sqlx::Error>

{
    let res = sqlx::query_as::<_, FancyDbObj>(r"SELECT * FROM fancy WHERE address = $1;")
        .bind(address)
        .fetch_optional(conn)
        .await?;
    Ok(res)
}

pub async fn fancy_update_owner<'c>(
    conn: &mut DbExecutor<'c>,
    address: DbAddress,
    owner: String,
) -> Result<(), sqlx::Error>

{
    let _res = sqlx::query(r"UPDATE fancy SET owner = $1 WHERE address = $2;")
        .bind(owner)
        .bind(address)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn fancy_update_score<'c>(
    conn: &mut DbExecutor<'c>,
    address: DbAddress,
    score: f64,
    price: i64,
    category: &str,
) -> Result<(), sqlx::Error>

{
    let _res =
        sqlx::query(r"UPDATE fancy SET score = $1, price = $2, category = $3 WHERE address = $4;")
            .bind(score)
            .bind(price)
            .bind(category)
            .bind(address)
            .execute(conn)
            .await?;
    Ok(())
}

pub async fn fancy_get_miner_info<'c>(
    conn: &mut DbExecutor<'c>,
    miner_info_uid: &str,
) -> Result<Option<MinerDbObj>, sqlx::Error>
{
    let res = sqlx::query_as::<_, MinerDbObj>(r"SELECT * FROM miner_info WHERE uid = $1;")
        .bind(miner_info_uid)
        .fetch_optional(conn.exec())
        .await?;
    Ok(res)
}

pub async fn fancy_insert_miner_info<'c>(
    conn: &mut DbExecutor<'c>,
    miner_info: MinerDbObj,
) -> Result<MinerDbObj, sqlx::Error>
{
    let res = sqlx::query_as::<_, MinerDbObj>(
        r"INSERT INTO miner_info (uid, prov_name, prov_node_id, prov_reward_addr, prov_extra_info)
VALUES ($1, $2, $3, $4, $5) RETURNING *;",
    )
    .bind(&miner_info.uid)
    .bind(&miner_info.prov_name)
    .bind(miner_info.prov_node_id)
    .bind(miner_info.prov_reward_addr)
    .bind(&miner_info.prov_extra_info)
    .fetch_one(conn.exec())
    .await?;
    Ok(res)
}

pub async fn fancy_get_job_info<'c>(conn: &mut DbExecutor<'c>, uid: &str) -> Result<JobDbObj, sqlx::Error>
{
    let res = sqlx::query_as::<_, JobDbObj>(r"SELECT * FROM job_info WHERE uid = $1;")
        .bind(uid)
        .fetch_one(conn.exec())
        .await?;
    Ok(res)
}

pub async fn fancy_insert_job_info<'c>(
    conn: &mut DbExecutor<'c>,
    job_info: JobDbObj,
) -> Result<JobDbObj, sqlx::Error>

{
    let res = sqlx::query_as::<_, JobDbObj>(
        r"INSERT INTO job_info (uid, cruncher_ver, started_at, finished_at, requestor_id, hashes_accepted, hashes_reported, cost_reported, miner, job_extra_info)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *;",
    )
        .bind(&job_info.uid)
        .bind(&job_info.cruncher_ver)
        .bind(job_info.started_at)
        .bind(job_info.finished_at)
        .bind(job_info.requestor_id)
        .bind(job_info.hashes_accepted)
        .bind(job_info.hashes_reported)
        .bind(job_info.cost_reported)
        .bind(&job_info.miner)
        .bind(&job_info.job_extra_info)
        .fetch_one(conn.exec())
        .await?;
    Ok(res)
}

pub async fn fancy_update_job<'c>(
    conn: &mut DbExecutor<'c>,
    job_uid: &str,
    hashes_accepted: f64,
    hashes_reported: f64,
    cost_reported: f64,
) -> Result<(), sqlx::Error>
{
    let _res = sqlx::query(
        r"UPDATE job_info SET hashes_accepted = $1, hashes_reported = $2, cost_reported = $3 WHERE uid = $4;",
    )
    .bind(hashes_accepted)
    .bind(hashes_reported)
    .bind(cost_reported)
    .bind(job_uid)
    .execute(conn.exec())
    .await?;
    Ok(())
}

pub async fn fancy_finish_job<'c>(conn: &mut DbExecutor<'c>, job_uid: &str) -> Result<(), sqlx::Error>
{
    let _res = sqlx::query(r"UPDATE job_info SET finished_at = $1 WHERE uid = $2;")
        .bind(Utc::now().naive_utc())
        .bind(job_uid)
        .execute(conn.exec())
        .await?;
    Ok(())
}
