use std::convert::identity;

use futures_util::{future::BoxFuture, FutureExt};
use mysql_async::{
    prelude::{FromRow, FromValue, ToConnection, WithParams},
    BinaryProtocol, Params, QueryResult, Value,
};
use mysql_common::row::ColumnIndex;

use crate::{query::Args, Query};

impl<'a> Into<Params> for Args<'a> {
    fn into(self) -> Params {
        Params::Positional(
            self.0
                .into_iter()
                .map(|a| match a {
                    crate::Type::Bool(a) => a.into(),
                    crate::Type::Int8(a) => a.into(),
                    crate::Type::Int16(a) => a.into(),
                    crate::Type::Int32(a) => a.into(),
                    crate::Type::Int64(a) => a.into(),
                    crate::Type::Isize(a) => a.into(),
                    crate::Type::UInt8(a) => a.into(),
                    crate::Type::UInt16(a) => a.into(),
                    crate::Type::UInt32(a) => a.into(),
                    crate::Type::UInt64(a) => a.into(),
                    crate::Type::Usize(a) => a.into(),
                    crate::Type::Float(a) => a.into(),
                    crate::Type::Double(a) => a.into(),
                    crate::Type::Null => Value::NULL,
                    crate::Type::String(a) => a.as_ref().into(),

                    #[cfg(feature = "uuid")]
                    crate::Type::Uuid(a) => a.to_string().into(),
                })
                .collect(),
        )
    }
}

pub trait MysqlQueryExt<'a, 't: 'a>: Sized + Send + 'a {
    /// Runs the given query via the underlying MySQL connection and returns a
    /// future that resolves to the raw result.
    fn get_raw<C>(
        self,
        con: C,
    ) -> BoxFuture<'a, Result<QueryResult<'a, 't, BinaryProtocol>, crate::Error>>
    where
        C: ToConnection<'a, 't> + 'a;

    /// Runs the given query via the underlying MySQL connection and just
    /// returns the number of affected rows.
    fn execute<C>(self, con: C) -> BoxFuture<'a, Result<u64, crate::Error>>
    where
        C: ToConnection<'a, 't> + 'a,
    {
        async move { Ok(self.get_raw(con).await?.affected_rows()) }.boxed()
    }

    /// Runs the query and returns an iterator of items that can be constructed
    /// from a single row.
    fn get<C, T>(self, con: C) -> BoxFuture<'a, Result<Vec<T>, crate::Error>>
    where
        C: ToConnection<'a, 't> + 'a,
        T: FromRow + Send,
    {
        async move { Ok(self.get_raw(con).await?.map(|r| T::from_row(r)).await?) }.boxed()
    }

    /// Runs the query and returns a single (the first) item if the result set.
    fn first<C, T>(self, con: C) -> BoxFuture<'a, Result<Option<T>, crate::Error>>
    where
        C: ToConnection<'a, 't> + 'a,
        T: FromRow + Send,
    {
        async move { Ok(self.get(con).await?.into_iter().nth(0)) }.boxed()
    }

    /// Runs the query and returns an iterator over all of the values from the
    /// specified column.
    fn pluck<C, T, I>(self, con: C, idx: I) -> BoxFuture<'a, Result<Vec<T>, crate::Error>>
    where
        C: ToConnection<'a, 't> + 'a,
        I: ColumnIndex + ToOwned<Owned = I> + Send + Sync + 'a,
        T: FromValue + Send,
    {
        async move {
            Ok(self
                .get_raw(con)
                .await?
                .map(|r| r.get::<T, I>(idx.to_owned()))
                .await?
                .into_iter()
                .filter_map(identity)
                .collect::<Vec<_>>())
        }
        .boxed()
    }

    /// Runs the query and returns an iterator over all values from the first
    /// column of all rows.
    fn all_values<C, T>(self, con: C) -> BoxFuture<'a, Result<Vec<T>, crate::Error>>
    where
        C: ToConnection<'a, 't> + 'a,
        T: FromValue + Send,
    {
        async move { Ok(self.pluck(con, 0).await?) }.boxed()
    }

    /// Runs the query and returns the value of the first column of the first
    /// row.
    fn value<C, T>(self, con: C) -> BoxFuture<'a, Result<Option<T>, crate::Error>>
    where
        C: ToConnection<'a, 't> + 'a,
        T: FromValue + Send,
    {
        // TODO: Return Result<T> with Error::UnexpectedRowCount when there was no result
        async move { Ok(self.all_values(con).await?.into_iter().nth(0)) }.boxed()
    }
}

impl<'a, 't: 'a> MysqlQueryExt<'a, 't> for Query<'a> {
    fn get_raw<C>(
        self,
        con: C,
    ) -> BoxFuture<'a, Result<QueryResult<'a, 't, BinaryProtocol>, crate::Error>>
    where
        C: ToConnection<'a, 't> + 'a,
    {
        async move {
            Ok(mysql_async::prelude::Query::run(
                self.build().map(|(query, args)| query.with(args))?,
                con,
            )
            .await?)
        }
        .boxed()
    }
}
