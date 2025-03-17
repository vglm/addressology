use crate::db::model::{
    ContractFactoryDbObject, FancyDbObj, FancyProviderDbObj, JobDbObj, JobMinerDbReadObj,
    MinerDbObj, PublicKeyBaseDbObject,
};
use crate::db::utils::get_min_time;
use crate::types::DbAddress;
use chrono::{NaiveDateTime, Utc};
use sqlx::types::Uuid;
use sqlx::{Executor, PgPool, Postgres, Transaction};

pub async fn insert_fancy_obj<'c, E>(
    conn: E,
    fancy_data: FancyDbObj,
) -> Result<FancyDbObj, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let res = sqlx::query_as::<_, FancyDbObj>(
        r"INSERT INTO fancy
(address, salt, factory, created, score, job_id, owner_id, price, category, public_key_base)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *;
",
    )
    .bind(fancy_data.address)
    .bind(&fancy_data.salt)
    .bind(fancy_data.factory)
    .bind(fancy_data.created)
    .bind(fancy_data.score)
    .bind(fancy_data.job_id)
    .bind(fancy_data.owner_id)
    .bind(fancy_data.price)
    .bind(&fancy_data.category)
    .bind(&fancy_data.public_key_base)
    .fetch_one(conn)
    .await?;
    Ok(res)
}

pub async fn fancy_list_all(
    conn: &PgPool,
    since: Option<NaiveDateTime>,
) -> Result<Vec<FancyDbObj>, sqlx::Error> {
    let res = sqlx::query_as::<_, FancyDbObj>(r"SELECT * FROM fancy WHERE created > $1;")
        .bind(since.unwrap_or(get_min_time()))
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
    User(Uuid),
}
pub enum PublicKeyFilter {
    All,
    Selected(String),
    OnlyNull,
}

pub async fn get_public_key_list<'c, E>(
    conn: E,
    user_id: Option<Uuid>,
) -> Result<Vec<PublicKeyBaseDbObject>, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let where_clause = match user_id {
        Some(uid) => format!("WHERE user_id = '{}' OR user_id is NULL", uid),
        None => "".to_string(),
    };

    let res = sqlx::query_as::<_, PublicKeyBaseDbObject>(&format!(
        r"SELECT * FROM public_key_base {where_clause};"
    ))
    .fetch_all(conn)
    .await?;
    Ok(res)
}

pub async fn get_or_insert_public_key(
    conn: &mut Transaction<'_, Postgres>,
    public_key_base: &str,
) -> Result<PublicKeyBaseDbObject, sqlx::Error> {
    //select first
    let res = sqlx::query_as::<_, PublicKeyBaseDbObject>(
        r"SELECT * FROM public_key_base WHERE hex = $1;",
    )
    .bind(public_key_base)
    .fetch_optional(&mut **conn)
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
        .fetch_one(&mut **conn)
        .await?;
        Ok(res)
    }
}

pub async fn get_or_insert_factory(
    conn: &mut Transaction<'_, Postgres>,
    factory_address: DbAddress,
) -> Result<ContractFactoryDbObject, sqlx::Error> {
    //select first
    let res = sqlx::query_as::<_, ContractFactoryDbObject>(
        r"SELECT * FROM contract_factory WHERE address = $1;",
    )
    .bind(factory_address)
    .fetch_optional(&mut **conn)
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
        .fetch_one(&mut **conn)
        .await?;
        Ok(res)
    }
}

pub async fn fancy_list<'c, E>(
    conn: E,
    category: Option<String>,
    order_by: FancyOrderBy,
    reserved: ReservedStatus,
    since: Option<NaiveDateTime>,
    public_key_base: PublicKeyFilter,
    limit: i64,
) -> Result<Vec<FancyProviderDbObj>, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
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
    .fetch_all(conn)
    .await?;
    Ok(res)
}

pub enum FancyJobOrderBy {
    Date,
}

pub enum FancyJobStatus {
    All,
    Active,
    Finished,
}

pub async fn fancy_job_list<'c, E>(
    conn: E,
    order_by: FancyJobOrderBy,
    since: Option<NaiveDateTime>,
    status: FancyJobStatus,
    requestor_id: Option<DbAddress>,
    limit: i64,
) -> Result<Vec<JobMinerDbReadObj>, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let order_by = match order_by {
        FancyJobOrderBy::Date => "updated_at",
    };

    let requestor_id_condition = match requestor_id {
        Some(id) => format!("requestor_id = '{}'", id),
        None => "".to_string(),
    };

    let created_condition = match since {
        Some(since) => format!("updated_at > '{}'", since.format("%Y-%m-%d %H:%M:%S")),
        None => "".to_string(),
    };

    let status_condition = match status {
        FancyJobStatus::All => "".to_string(),
        FancyJobStatus::Active => "finished_at is NULL".to_string(),
        FancyJobStatus::Finished => "finished_at is NOT NULL".to_string(),
    };

    let where_clause = [
        "1=1".to_string(),
        requestor_id_condition,
        created_condition,
        status_condition,
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

    let res = sqlx::query_as::<_, JobMinerDbReadObj>(
        format!(
            r"SELECT
                cruncher_ver,
                started_at,
                updated_at,
                finished_at,
                requestor_id,
                hashes_accepted,
                hashes_reported,
                entries_accepted,
                entries_rejected,
                cost_reported,
                job_extra_info,
                prov_name,
                prov_node_id,
                prov_reward_addr,
                prov_extra_info
            FROM job_info as ji JOIN miner_info as mi on ji.miner=mi.uid
            WHERE {where_clause}
            ORDER BY {order_by_clause}
            {limit_clause}"
        )
        .as_str(),
    )
    .fetch_all(conn)
    .await?;
    Ok(res)
}

pub async fn fancy_get_by_address<'c, E>(
    conn: E,
    address: DbAddress,
) -> Result<Option<FancyDbObj>, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let res = sqlx::query_as::<_, FancyDbObj>(r"SELECT * FROM fancy WHERE address = $1;")
        .bind(address)
        .fetch_optional(conn)
        .await?;
    Ok(res)
}

pub async fn fancy_update_owner<'c, E>(
    conn: E,
    address: DbAddress,
    owner: Uuid,
) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let _res = sqlx::query(r"UPDATE fancy SET owner = $1 WHERE address = $2;")
        .bind(owner)
        .bind(address)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn fancy_update_score<'c, E>(
    conn: E,
    address: DbAddress,
    score: f64,
    price: i64,
    category: &str,
) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
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

pub async fn fancy_get_miner_info<'c, E>(
    conn: E,
    miner_info_uid: &str,
) -> Result<Option<MinerDbObj>, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let res = sqlx::query_as::<_, MinerDbObj>(r"SELECT * FROM miner_info WHERE uid = $1;")
        .bind(miner_info_uid)
        .fetch_optional(conn)
        .await?;
    Ok(res)
}

pub async fn fancy_insert_miner_info<'c, E>(
    conn: E,
    miner_info: MinerDbObj,
) -> Result<MinerDbObj, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
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
    .fetch_one(conn)
    .await?;
    Ok(res)
}

pub async fn fancy_get_job_info<'c, E>(conn: E, uid: Uuid) -> Result<JobDbObj, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let res = sqlx::query_as::<_, JobDbObj>(r"SELECT * FROM job_info WHERE uid = $1;")
        .bind(uid)
        .fetch_one(conn)
        .await?;
    Ok(res)
}

pub async fn fancy_insert_job_info<'c, E>(
    conn: E,
    job_info: JobDbObj,
) -> Result<JobDbObj, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let res = sqlx::query_as::<_, JobDbObj>(
        r"INSERT INTO job_info (uid, cruncher_ver, started_at, updated_at, finished_at, requestor_id, hashes_accepted, hashes_reported, entries_accepted, entries_rejected, cost_reported, miner, job_extra_info)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING *;",
    )
        .bind(job_info.uid)
        .bind(&job_info.cruncher_ver)
        .bind(job_info.started_at)
        .bind(job_info.updated_at)
        .bind(job_info.finished_at)
        .bind(job_info.requestor_id)
        .bind(job_info.hashes_accepted)
        .bind(job_info.hashes_reported)
        .bind(job_info.entries_accepted)
        .bind(job_info.entries_rejected)
        .bind(job_info.cost_reported)
        .bind(&job_info.miner)
        .bind(&job_info.job_extra_info)
        .fetch_one(conn)
        .await?;
    Ok(res)
}

pub async fn fancy_update_job<'c, E>(
    conn: E,
    job_uid: Uuid,
    hashes_accepted: f64,
    hashes_reported: f64,
    entries_accepted: i64,
    entries_rejected: i64,
    cost_reported: f64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let _res = sqlx::query(
        r"UPDATE job_info SET
                  hashes_accepted = $1,
                  hashes_reported = $2,
                  entries_accepted = $3,
                  entries_rejected = $4,
                  cost_reported = $5,
                  updated_at = $6
              WHERE uid = $7;",
    )
    .bind(hashes_accepted)
    .bind(hashes_reported)
    .bind(entries_accepted)
    .bind(entries_rejected)
    .bind(cost_reported)
    .bind(Utc::now().naive_utc())
    .bind(job_uid)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn fancy_finish_job<'c, E>(conn: E, job_uid: Uuid) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let _res = sqlx::query(r"UPDATE job_info SET finished_at = $1 WHERE uid = $2;")
        .bind(Utc::now().naive_utc())
        .bind(job_uid)
        .execute(conn)
        .await?;
    Ok(())
}
