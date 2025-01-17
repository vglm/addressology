use crate::db::model::FancyDbObj;
use crate::types::DbAddress;
use sqlx::{Executor, Sqlite, SqlitePool};

pub async fn insert_fancy_obj(
    conn: &SqlitePool,
    fancy_data: FancyDbObj,
) -> Result<FancyDbObj, sqlx::Error> {
    let res = sqlx::query_as::<_, FancyDbObj>(
        r"INSERT INTO fancy
(address, salt, factory, created, score, miner)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *;
",
    )
    .bind(fancy_data.address)
    .bind(&fancy_data.salt)
    .bind(fancy_data.factory)
    .bind(fancy_data.created)
    .bind(fancy_data.score)
    .bind(&fancy_data.miner)
    .bind(&fancy_data.owner)
    .bind(fancy_data.price)
    .fetch_one(conn)
    .await?;
    Ok(res)
}

pub async fn list_all_free(conn: &SqlitePool) -> Result<Vec<FancyDbObj>, sqlx::Error> {
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

pub async fn fancy_list_best_score(conn: &SqlitePool) -> Result<Vec<FancyDbObj>, sqlx::Error> {
    let res = sqlx::query_as::<_, FancyDbObj>(
        r"SELECT * FROM fancy WHERE owner is NULL ORDER BY score DESC LIMIT 100;",
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
