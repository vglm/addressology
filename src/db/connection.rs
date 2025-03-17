use crate::error::AddressologyError;
use crate::error::*;
use sqlx::migrate::{MigrateError, Migrator};

use sqlx::PgPool;
use std::env;
use std::path::Path;
use crate::err_custom_create;

static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn create_pg_connection() -> Result<PgPool, AddressologyError> {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&db_url).await.map_err(
        |e| err_custom_create!("Failed to connect to database: {}", e)
    )?;
    Ok(pool)
}
