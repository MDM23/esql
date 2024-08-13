mod database;
mod query;
mod types;

pub use query::{
    from, having, r#where, r#where_in, raw, select, trusted, wh, wh_in, Args, Having, Query,
    TrustedString, Where,
};
pub use types::Type;

#[cfg(feature = "mysql-async")]
pub use database::mysql::MysqlQueryExt;

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
