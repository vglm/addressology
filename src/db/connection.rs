use crate::error::AddressologyError;
use sqlx::migrate::Migrator;

use crate::err_custom_create;
use sqlx::PgPool;
use std::env;

static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn create_pg_connection(run_migrations: bool) -> Result<PgPool, AddressologyError> {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&db_url)
        .await
        .map_err(|e| err_custom_create!("Failed to connect to database: {}", e))?;

    if run_migrations {
        MIGRATOR
            .run(&pool)
            .await
            .map_err(|e| err_custom_create!("Failed to run migrations: {}", e))?;
    }
    Ok(pool)
}
