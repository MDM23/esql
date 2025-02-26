use std::{fmt::Display, future::Future, pin::pin};

use futures_util::StreamExt as _;
use qp_postgres::PgPool;
use serde::Deserialize;
use tokio_postgres::{
    row::RowIndex,
    tls::{MakeTlsConnect, TlsConnect},
    types::{private::BytesMut, FromSqlOwned, ToSql},
    Client, Row, RowStream, Socket, Transaction,
};

use crate::{
    query::{ArgFormat, Query},
    serde::{Error, PgRow},
    Type,
};

impl ToSql for Type<'_> {
    fn to_sql(
        &self,
        ty: &tokio_postgres::types::Type,
        out: &mut BytesMut,
    ) -> Result<tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        match self {
            Type::Bool(a) => a.to_sql(ty, out),
            Type::Int8(a) => a.to_sql(ty, out),
            Type::Int16(a) => a.to_sql(ty, out),
            Type::Int32(a) => a.to_sql(ty, out),
            Type::Int64(a) => a.to_sql(ty, out),
            Type::Isize(a) => (*a as i64).to_sql(ty, out),
            Type::UInt8(a) => (*a as i16).to_sql(ty, out),
            Type::UInt16(a) => (*a as u32).to_sql(ty, out),
            Type::UInt32(a) => a.to_sql(ty, out),
            Type::UInt64(a) => (*a as u32).to_sql(ty, out),
            Type::Usize(a) => (*a as u32).to_sql(ty, out),
            Type::Float(a) => a.to_sql(ty, out),
            Type::Double(a) => a.to_sql(ty, out),
            Type::Null => None::<Option<bool>>.to_sql(ty, out),
            Type::String(a) => a.to_sql(ty, out),

            #[cfg(feature = "time")]
            Type::OffsetDateTime(a) => a.to_sql(ty, out),

            #[cfg(feature = "uuid")]
            Type::Uuid(a) => a.to_sql(ty, out),
        }
    }

    fn accepts(_: &tokio_postgres::types::Type) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn to_sql_checked(
        &self,
        ty: &tokio_postgres::types::Type,
        out: &mut BytesMut,
    ) -> Result<tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        self.to_sql(ty, out)
    }
}

fn slice_iter<'a>(s: &'a [Type<'a>]) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
    s.iter().map(|s| s as _)
}

pub trait PgQueryExt<'a, C>
where
    Self: Sized,
{
    fn get_raw(self, con: &C) -> impl Future<Output = Result<RowStream, crate::Error>>;
    fn execute(self, con: &C) -> impl Future<Output = Result<u64, crate::Error>>;

    fn get<T>(self, con: &C) -> impl Future<Output = Result<Vec<T>, crate::Error>>
    where
        T: for<'de> Deserialize<'de>,
    {
        async move {
            self.get_raw(con)
                .await?
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .map(|row| {
                    if let Ok(r) = row {
                        Self::deserialize_row(&r).map_err(|_| crate::Error::FromRowError)
                    } else {
                        Err(crate::Error::FromRowError)
                    }
                })
                .collect()
        }
    }

    fn first<T>(self, con: &C) -> impl Future<Output = Result<Option<T>, crate::Error>>
    where
        T: for<'de> Deserialize<'de>,
    {
        async move {
            match pin!(self.get_raw(con).await?).next().await {
                None => Ok(None),
                Some(row) => {
                    if let Ok(r) = row {
                        Ok(Some(
                            Self::deserialize_row(&r).map_err(|_| crate::Error::FromRowError)?,
                        ))
                    } else {
                        Err(crate::Error::FromRowError)
                    }
                }
            }
        }
    }

    fn pluck<T, I>(self, con: &C, idx: I) -> impl Future<Output = Result<Vec<T>, crate::Error>>
    where
        T: FromSqlOwned,
        I: RowIndex + ToOwned<Owned = I> + Display,
    {
        async move {
            self.get_raw(con)
                .await?
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .map(|row| {
                    if let Ok(r) = row {
                        r.try_get(idx.to_owned())
                            .map_err(|_| crate::Error::FromRowError)
                    } else {
                        Err(crate::Error::FromRowError)
                    }
                })
                .collect()
        }
    }

    fn values<T>(self, con: &C) -> impl Future<Output = Result<Vec<T>, crate::Error>>
    where
        T: FromSqlOwned,
    {
        self.pluck(con, 0)
    }

    fn value<T>(self, con: &C) -> impl Future<Output = Result<Option<T>, crate::Error>>
    where
        T: FromSqlOwned,
    {
        async move {
            match pin!(self.get_raw(con).await?).next().await {
                None => Ok(None),
                Some(row) => {
                    if let Ok(r) = row {
                        Ok(Some(r.try_get(0).map_err(|_| crate::Error::FromRowError)?))
                    } else {
                        Err(crate::Error::FromRowError)
                    }
                }
            }
        }
    }

    fn deserialize_row<T: for<'de> Deserialize<'de>>(row: &Row) -> Result<T, Error> {
        Deserialize::deserialize(PgRow::from(row))
    }
}

impl<'a, S> PgQueryExt<'a, Client> for Query<'a, S> {
    fn get_raw(self, con: &Client) -> impl Future<Output = Result<RowStream, crate::Error>> {
        async move {
            let (statement, args) = self.build(ArgFormat::Indexed);

            con.query_raw(&statement, slice_iter(&args))
                .await
                .map_err(|e| e.into())
        }
    }

    fn execute(self, con: &Client) -> impl Future<Output = Result<u64, crate::Error>> {
        async move {
            let (statement, args) = self.build(ArgFormat::Indexed);

            con.execute_raw(&statement, slice_iter(&args))
                .await
                .map_err(|e| e.into())
        }
    }
}

impl<'a, S> PgQueryExt<'a, Transaction<'a>> for Query<'a, S> {
    fn get_raw(
        self,
        con: &Transaction<'a>,
    ) -> impl Future<Output = Result<RowStream, crate::Error>> {
        async move {
            let (statement, args) = self.build(ArgFormat::Indexed);

            con.query_raw(&statement, slice_iter(&args))
                .await
                .map_err(|e| e.into())
        }
    }

    fn execute(self, con: &Transaction<'a>) -> impl Future<Output = Result<u64, crate::Error>> {
        async move {
            let (statement, args) = self.build(ArgFormat::Indexed);

            con.execute_raw(&statement, slice_iter(&args))
                .await
                .map_err(|e| e.into())
        }
    }
}

#[cfg(feature = "qp-postgres")]
impl<'a, S, T> PgQueryExt<'a, PgPool<T>> for Query<'a, S>
where
    T: MakeTlsConnect<Socket> + Clone + Send + Sync,
    T::Stream: Send + Sync + 'static,
    T::TlsConnect: Send + Sync,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    fn get_raw(self, con: &PgPool<T>) -> impl Future<Output = Result<RowStream, crate::Error>> {
        async move {
            let (statement, args) = self.build(ArgFormat::Indexed);

            con.acquire()
                .await?
                .query_raw(&statement, slice_iter(&args))
                .await
                .map_err(|e| e.into())
        }
    }

    fn execute(self, con: &PgPool<T>) -> impl Future<Output = Result<u64, crate::Error>> {
        async move {
            let (statement, args) = self.build(ArgFormat::Indexed);

            con.acquire()
                .await?
                .execute_raw(&statement, slice_iter(&args))
                .await
                .map_err(|e| e.into())
        }
    }
}
