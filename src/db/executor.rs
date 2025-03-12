
use futures_core::{future::BoxFuture, stream::BoxStream};
use sqlx::{Database, Error, Executor, Pool, Sqlite, SqliteConnection, Transaction};

pub enum DbExecutor<'c> {
    Tx(Transaction<'c, Sqlite>),
    Pool(&'c Pool<Sqlite>),
}

#[derive(Debug)]
pub enum DbEitherExecutor<'e> {
    Conn(&'e mut SqliteConnection),
    Pool(&'e Pool<Sqlite>),
}

impl<'c> DbExecutor<'c> {
    pub fn from(pool: &'c Pool<Sqlite>) -> Self {
        DbExecutor::Pool(pool)
    }
    pub fn from_tx(tx: Transaction<'c, Sqlite>) -> Self {
        DbExecutor::Tx(tx)
    }
    pub async fn commit(self: Self) -> Result<(), Error> {
        match self {
            DbExecutor::Tx(tx) => {
                tx.commit().await
            }
            _ => {
                log::warn!("Trying to commit a non-transaction");
                Ok(())
            }
        }
    }
    pub async fn rollback(self: Self) -> Result<(), Error> {
        match self {
            DbExecutor::Tx(tx) => {
                tx.rollback().await
            }
            _ => {
                log::warn!("Trying to rollback a non-transaction");
                Ok(())
            }
        }
    }
    pub fn exec<'e>(&'e mut self) -> DbEitherExecutor<'e> where 'c: 'e {
        match self {
            DbExecutor::Tx(tx) => DbEitherExecutor::Conn(&mut *tx),
            DbExecutor::Pool(pool) => DbEitherExecutor::Pool(pool),
        }
    }
}

impl<'c> Executor<'c> for DbEitherExecutor<'c> {
    type Database = Sqlite;

    fn fetch_many<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> BoxStream<
        'e,
        Result<
            sqlx::Either<
                <Self::Database as Database>::QueryResult,
                <Self::Database as Database>::Row,
            >,
            sqlx::Error,
        >,
    >
    where
        'c: 'e,
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        match self {
            DbEitherExecutor::Conn(conn) => conn.fetch_many(query),
            DbEitherExecutor::Pool(pool) => pool.fetch_many(query),
        }
    }

    fn fetch_optional<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<Option<<Self::Database as Database>::Row>, sqlx::Error>>
    where
        'c: 'e,
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        match self {
            DbEitherExecutor::Conn(conn) => conn.fetch_optional(query),
            DbEitherExecutor::Pool(pool) => pool.fetch_optional(query),
        }
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [<Self::Database as Database>::TypeInfo],
    ) -> BoxFuture<'e, Result<<Self::Database as Database>::Statement<'q>, sqlx::Error>>
    where
        'c: 'e,
    {
        match self {
            DbEitherExecutor::Conn(conn) => conn.prepare_with(sql, parameters),
            DbEitherExecutor::Pool(pool) => pool.prepare_with(sql, parameters),
        }
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>>
    where
        'c: 'e,
    {
        match self {
            DbEitherExecutor::Conn(conn) => conn.describe(sql),
            DbEitherExecutor::Pool(pool) => pool.describe(sql),
        }
    }
}
