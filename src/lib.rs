mod database;
mod query;
mod types;

pub use query::{expr, in_expr, query, trusted, ArgFormat, Expr, Query, TrustedString};

pub use types::Type;

// #[cfg(feature = "mysql-async")]
// pub use database::mysql::MysqlQueryExt;

#[cfg(feature = "tokio-postgres")]
pub use database::pg::PgQueryExt;

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

    #[error("conversion from a row failed")]
    FromRowError,
}
