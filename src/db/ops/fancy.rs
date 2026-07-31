use crate::db::model::{
    ContractFactoryDbObject, FancyCategoryStatDbObj, FancyDbObj, FancyProviderDbObj, JobDbObj,
    JobMinerDbReadObj, MinerDbObj, PublicKeyBaseDbObject,
};
use crate::db::utils::get_min_time;
use crate::types::DbAddress;
use chrono::{NaiveDateTime, Utc};
use sqlx::types::Uuid;
use sqlx::{Executor, PgPool, Postgres, QueryBuilder, Transaction};

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
            r"INSERT INTO public_key_base (uid, hex, added) VALUES ($1, $2, $3) RETURNING *;",
        )
        .bind(Uuid::new_v4())
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
            r"INSERT INTO contract_factory (uid, address, added) VALUES ($1, $2, $3) RETURNING *;",
        )
        .bind(Uuid::new_v4())
        .bind(factory_address)
        .bind(Utc::now().naive_utc())
        .fetch_one(&mut **conn)
        .await?;
        Ok(res)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternMode {
    Prefix,
    Contains,
    Suffix,
}

pub struct FancySearchParams {
    /// Lowercase hex digits only (without 0x prefix), pre-validated by the caller.
    pub pattern: Option<String>,
    pub pattern_mode: PatternMode,
    pub category: Option<String>,
    pub order_by: FancyOrderBy,
    pub order_ascending: bool,
    pub reserved: ReservedStatus,
    pub since: Option<NaiveDateTime>,
    pub provider_id: Option<String>,
    pub public_key_base: PublicKeyFilter,
    pub limit: i64,
    pub offset: i64,
}

pub struct FancySearchResult {
    pub entries: Vec<FancyProviderDbObj>,
    pub total: i64,
    pub total_capped: bool,
}

/// Highest exact total reported for pattern searches; above this the count is reported as capped.
const SEARCH_COUNT_CAP: i64 = 100_000;

fn push_fancy_conditions(builder: &mut QueryBuilder<'_, Postgres>, params: &FancySearchParams) {
    builder.push(" WHERE 1=1");
    match &params.reserved {
        ReservedStatus::All => {}
        ReservedStatus::Reserved => {
            builder.push(" AND f.owner_id is NOT NULL");
        }
        ReservedStatus::NotReserved => {
            builder.push(" AND f.owner_id is NULL");
        }
        ReservedStatus::User(user) => {
            builder.push(" AND f.owner_id = ");
            builder.push_bind(*user);
        }
    }
    match &params.public_key_base {
        PublicKeyFilter::All => {}
        PublicKeyFilter::Selected(pk) => {
            builder.push(" AND f.public_key_base = ");
            builder.push_bind(pk.clone());
        }
        PublicKeyFilter::OnlyNull => {
            builder.push(" AND f.public_key_base is NULL");
        }
    }
    if let Some(category) = &params.category {
        builder.push(" AND f.category = ");
        builder.push_bind(category.clone());
    }
    if let Some(since) = &params.since {
        builder.push(" AND f.created > ");
        builder.push_bind(*since);
    }
    if let Some(pattern) = &params.pattern {
        let like = match params.pattern_mode {
            PatternMode::Prefix => format!("0x{}%", pattern),
            PatternMode::Contains => format!("%{}%", pattern),
            PatternMode::Suffix => format!("%{}", pattern),
        };
        builder.push(" AND f.address LIKE ");
        builder.push_bind(like);
    }
    if let Some(provider_id) = &params.provider_id {
        builder.push(" AND mi.prov_node_id = ");
        builder.push_bind(provider_id.clone());
    }
}

fn push_fancy_order(builder: &mut QueryBuilder<'_, Postgres>, params: &FancySearchParams) {
    let column = match params.order_by {
        FancyOrderBy::Score => "f.score",
        FancyOrderBy::Created => "f.created",
    };
    let direction = if params.order_ascending {
        "ASC"
    } else {
        "DESC"
    };
    builder.push(format!(" ORDER BY {} {}", column, direction));
}

const FANCY_JOINS: &str = " FROM fancy as f LEFT JOIN job_info as ji ON f.job_id=ji.uid LEFT JOIN miner_info as mi ON mi.uid=ji.miner";

async fn fancy_query_page<'c, E>(
    conn: E,
    params: &FancySearchParams,
) -> Result<Vec<FancyProviderDbObj>, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT f.*, mi.prov_name, mi.prov_node_id, mi.prov_reward_addr",
    );
    builder.push(FANCY_JOINS);
    push_fancy_conditions(&mut builder, params);
    push_fancy_order(&mut builder, params);
    if params.limit > 0 {
        builder.push(" LIMIT ");
        builder.push_bind(params.limit);
    }
    if params.offset > 0 {
        builder.push(" OFFSET ");
        builder.push_bind(params.offset);
    }
    builder
        .build_query_as::<FancyProviderDbObj>()
        .fetch_all(conn)
        .await
}

pub async fn fancy_search(
    conn: &PgPool,
    params: &FancySearchParams,
) -> Result<FancySearchResult, sqlx::Error> {
    let mut count_builder = QueryBuilder::<Postgres>::new("SELECT count(*)");
    if params.pattern.is_some() {
        // Counting arbitrary pattern matches can touch a large part of the table,
        // so give up on exactness above the cap to keep the query bounded.
        count_builder = QueryBuilder::<Postgres>::new("SELECT count(*) FROM (SELECT 1");
    }
    if params.provider_id.is_some() {
        count_builder.push(FANCY_JOINS);
    } else {
        count_builder.push(" FROM fancy as f");
    }
    push_fancy_conditions(&mut count_builder, params);
    if params.pattern.is_some() {
        count_builder.push(" LIMIT ");
        count_builder.push_bind(SEARCH_COUNT_CAP + 1);
        count_builder.push(") as capped");
    }
    let total = count_builder
        .build_query_scalar::<i64>()
        .fetch_one(conn)
        .await?;
    let total_capped = params.pattern.is_some() && total > SEARCH_COUNT_CAP;
    let total = if total_capped {
        SEARCH_COUNT_CAP
    } else {
        total
    };

    let entries = if !total_capped && params.offset >= total {
        vec![]
    } else {
        fancy_query_page(conn, params).await?
    };

    Ok(FancySearchResult {
        entries,
        total,
        total_capped,
    })
}

pub async fn fancy_stats(conn: &PgPool) -> Result<Vec<FancyCategoryStatDbObj>, sqlx::Error> {
    sqlx::query_as::<_, FancyCategoryStatDbObj>(
        r"SELECT category,
                 count(*) as total,
                 count(*) FILTER (WHERE owner_id is NULL) as free,
                 max(created) as newest
          FROM fancy GROUP BY category ORDER BY total DESC",
    )
    .fetch_all(conn)
    .await
}

pub async fn fancy_estimated_work_events(conn: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        r"SELECT count(*) FROM fancy WHERE category = 'leading_zeroes' AND score > 1e11",
    )
    .fetch_one(conn)
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn fancy_list<'c, E>(
    conn: E,
    category: Option<String>,
    order_by: FancyOrderBy,
    reserved: ReservedStatus,
    since: Option<NaiveDateTime>,
    provider_id: Option<String>,
    public_key_base: PublicKeyFilter,
    limit: i64,
) -> Result<Vec<FancyProviderDbObj>, sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let params = FancySearchParams {
        pattern: None,
        pattern_mode: PatternMode::Prefix,
        category,
        order_by,
        order_ascending: false,
        reserved,
        since,
        provider_id,
        public_key_base,
        limit,
        offset: 0,
    };
    fancy_query_page(conn, &params).await
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
    owner_id: Uuid,
) -> Result<(), sqlx::Error>
where
    E: Executor<'c, Database = Postgres>,
{
    let _res = sqlx::query(r"UPDATE fancy SET owner_id = $1 WHERE address = $2;")
        .bind(owner_id)
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
