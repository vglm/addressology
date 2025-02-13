use crate::db::model::{FancyDbObj, JobDbObj, MinerDbObj};
use crate::types::DbAddress;
use chrono::{DateTime, Utc};
use sqlx::{Executor, Sqlite, SqlitePool};

pub async fn insert_fancy_obj<'c, E>(
    conn: E,
    fancy_data: FancyDbObj,
) -> Result<FancyDbObj, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let res = sqlx::query_as::<_, FancyDbObj>(
        r"INSERT INTO fancy
(address, salt, factory, created, score, job, owner, price, category)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *;
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
    .fetch_one(conn)
    .await?;
    Ok(res)
}

pub async fn fancy_list_all(conn: &SqlitePool) -> Result<Vec<FancyDbObj>, sqlx::Error> {
    let res = sqlx::query_as::<_, FancyDbObj>(r"SELECT * FROM fancy")
        .fetch_all(conn)
        .await?;
    Ok(res)
}

pub async fn fancy_list_all_free(conn: &SqlitePool) -> Result<Vec<FancyDbObj>, sqlx::Error> {
    let res = sqlx::query_as::<_, FancyDbObj>(r"SELECT * FROM fancy WHERE owner is NULL;")
        .fetch_all(conn)
        .await?;
    Ok(res)
}

pub async fn fancy_list_newest(conn: &SqlitePool) -> Result<Vec<FancyDbObj>, sqlx::Error> {
    let res = sqlx::query_as::<_, FancyDbObj>(
        r"SELECT * FROM fancy WHERE owner is NULL ORDER BY created DESC LIMIT 100;",
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

pub async fn fancy_list_best_score(
    conn: &SqlitePool,
    category: Option<String>,
    order_by: FancyOrderBy,
    since: Option<DateTime<Utc>>,
    limit: i64,
) -> Result<Vec<FancyDbObj>, sqlx::Error> {
    let order_by = match order_by {
        FancyOrderBy::Score => "score",
        FancyOrderBy::Created => "created",
    };
    let res = sqlx::query_as::<_, FancyDbObj>(
        format!(
            r"SELECT *
            FROM fancy WHERE
                owner is NULL
                and category LIKE $2
                and created > $3
            ORDER BY {} DESC LIMIT $1;",
            order_by
        )
        .as_str(),
    )
    .bind(limit)
    .bind(category.unwrap_or("%".to_string()))
    .bind(
        since
            .map(|s| s.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or("2000-01-01 00:00:00".to_string()),
    )
    .bind(order_by)
    .fetch_all(conn)
    .await?;
    Ok(res)
}

pub async fn fancy_get_by_address<'c, E>(
    conn: E,
    address: DbAddress,
) -> Result<Option<FancyDbObj>, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
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
    owner: String,
) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
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
    E: Executor<'c, Database = Sqlite>,
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
    E: Executor<'c, Database = Sqlite>,
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
    E: Executor<'c, Database = Sqlite>,
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

pub async fn fancy_get_job_info<'c, E>(conn: E, uid: &str) -> Result<JobDbObj, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
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
    E: Executor<'c, Database = Sqlite>,
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
        .fetch_one(conn)
        .await?;
    Ok(res)
}

pub async fn fancy_update_job<'c, E>(
    conn: E,
    job_uid: &str,
    hashes_accepted: f64,
    hashes_reported: f64,
    cost_reported: f64,
) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let _res = sqlx::query(
        r"UPDATE job_info SET hashes_accepted = $1, hashes_reported = $2, cost_reported = $3 WHERE uid = $4;",
    )
    .bind(hashes_accepted)
    .bind(hashes_reported)
    .bind(cost_reported)
    .bind(job_uid)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn fancy_finish_job<'c, E>(conn: E, job_uid: &str) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let _res = sqlx::query(r"UPDATE job_info SET finished_at = $1 WHERE uid = $2;")
        .bind(Utc::now().naive_utc())
        .bind(job_uid)
        .execute(conn)
        .await?;
    Ok(())
}
