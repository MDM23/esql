mod database;
mod query;

pub use query::Query;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("query returned an unexpected number of rows")]
    UnexpectedRowCount,

    #[cfg(feature = "tokio-postgres")]
    #[error(transparent)]
    PostgresError(#[from] tokio_postgres::Error),

    #[cfg(feature = "mysql-async")]
    #[error(transparent)]
    MysqlError(#[from] mysql_async::Error),
}
