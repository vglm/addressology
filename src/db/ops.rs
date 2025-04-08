mod contract;
mod fancy;
mod user;

pub use contract::*;
pub use fancy::*;
pub use user::*;

use std::future::Future;
use std::time::Duration;

const LOCKED_TIMEOUT: Duration = Duration::from_secs(300);

#[allow(dead_code)]
///Usage example:
/// do_ops_until_not_locked(|| get_all_token_transfers(&conn, None)).await?;
/// Postgres database likes to return this error randomly, when used by more processes so it's good to
/// ignore and retry until success.
pub async fn do_db_operation<R, Fun, Fut>(operation: Fun) -> Result<R, sqlx::Error>
where
    Fun: Fn() -> Fut,
    Fut: Future<Output = Result<R, sqlx::Error>>,
{
    let instant = std::time::Instant::now();
    loop {
        let res = operation().await;
        if res.is_err() && instant.elapsed() > LOCKED_TIMEOUT {
            log::error!(
                "Database is locked for {} seconds. Aborting...",
                LOCKED_TIMEOUT.as_secs()
            );
        } else if let Err(err) = &res {
            if let Some(db) = err.as_database_error() {
                if db.message() == "database is locked" {
                    log::warn!(
                        "Database is locked for {:.1} seconds. Trying again...",
                        instant.elapsed().as_secs_f64()
                    );
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            }
        }
        break res;
    }
}
