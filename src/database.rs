#[cfg(feature = "tokio-postgres")]
mod __impl_tokio_postgres {
    use std::{fmt::Display, ops::Deref};

    use futures_util::StreamExt;
    use tokio_postgres::{row::RowIndex, types::FromSqlOwned, Row, RowStream};

    impl crate::query::Query {
        pub async fn get<T, DB>(self, db: &DB) -> Result<impl IntoIterator<Item = T>, crate::Error>
        where
            DB: Deref<Target = tokio_postgres::Client>,
            T: From<Row>,
        {
            Ok(self
                .get_raw(db)
                .await?
                .map(|r| Into::<T>::into(r.unwrap()))
                .collect::<Vec<_>>()
                .await)
        }

        pub async fn first<T, DB>(self, db: &DB) -> Result<Option<T>, crate::Error>
        where
            DB: Deref<Target = tokio_postgres::Client>,
            T: From<Row>,
        {
            Ok(self.get::<T, DB>(db).await?.into_iter().nth(0))
        }

        pub async fn pluck<T, DB, I>(
            self,
            db: &DB,
            idx: I,
        ) -> Result<impl IntoIterator<Item = T>, crate::Error>
        where
            DB: Deref<Target = tokio_postgres::Client>,
            I: RowIndex + Display + ToOwned<Owned = I>,
            T: FromSqlOwned,
        {
            Ok(self
                .get_raw(db)
                .await?
                .map(|r| r.unwrap().get::<I, T>(idx.to_owned()))
                .collect::<Vec<_>>()
                .await)
        }

        pub async fn value<T, DB>(self, db: &DB) -> Result<Option<T>, crate::Error>
        where
            DB: Deref<Target = tokio_postgres::Client>,
            T: FromSqlOwned,
        {
            Ok(self.all_values(db).await?.into_iter().nth(0))
        }

        pub async fn all_values<T, DB>(
            self,
            db: &DB,
        ) -> Result<impl IntoIterator<Item = T>, crate::Error>
        where
            DB: Deref<Target = tokio_postgres::Client>,
            T: FromSqlOwned,
        {
            Ok(self.pluck(db, 0).await?)
        }

        pub async fn execute<T, DB>(self, db: &DB) -> Result<u64, crate::Error>
        where
            DB: Deref<Target = tokio_postgres::Client>,
        {
            Ok(db
                .deref()
                .execute_raw(&self.to_string(), self.into_args().iter().map(Deref::deref))
                .await?)
        }

        pub async fn get_raw<DB>(self, db: &DB) -> Result<RowStream, crate::Error>
        where
            DB: Deref<Target = tokio_postgres::Client>,
        {
            Ok(db
                .deref()
                .query_raw(&self.to_string(), self.into_args().iter().map(Deref::deref))
                .await?)
        }
    }
}

#[cfg(feature = "mysql-async")]
mod __impl_mysql_async {
    use mysql_async::{
        prelude::{FromRow, FromValue, WithParams},
        BinaryProtocol, QueryResult,
    };
    use mysql_common::row::ColumnIndex;

    use crate::query::Query;

    impl Query {
        pub async fn get<'a, 't: 'a, T, DB>(
            self,
            db: DB,
        ) -> Result<impl IntoIterator<Item = T>, crate::Error>
        where
            DB: mysql_async::prelude::ToConnection<'a, 't> + 'a,
            T: FromRow,
        {
            Ok(self.get_raw(db).await?.map(|r| T::from_row(r)).await?)
        }

        pub async fn first<'a, 't: 'a, T, DB>(self, db: DB) -> Result<Option<T>, crate::Error>
        where
            DB: mysql_async::prelude::ToConnection<'a, 't> + 'a,
            T: FromRow,
        {
            Ok(self.get::<T, DB>(db).await?.into_iter().nth(0))
        }

        pub async fn pluck<'a, 't: 'a, T, DB, I>(
            self,
            db: DB,
            idx: I,
        ) -> Result<impl IntoIterator<Item = T>, crate::Error>
        where
            DB: mysql_async::prelude::ToConnection<'a, 't> + 'a,
            I: ColumnIndex + ToOwned<Owned = I>,
            T: FromValue,
        {
            Ok(self
                .get_raw(db)
                .await?
                .map(|r| r.get::<T, I>(idx.to_owned()).unwrap())
                .await?)
        }

        pub async fn value<'a, 't: 'a, T, DB>(self, db: DB) -> Result<Option<T>, crate::Error>
        where
            DB: mysql_async::prelude::ToConnection<'a, 't> + 'a,
            T: FromValue,
        {
            Ok(self.all_values(db).await?.into_iter().nth(0))
        }

        pub async fn all_values<'a, 't: 'a, T, DB>(
            self,
            db: DB,
        ) -> Result<impl IntoIterator<Item = T>, crate::Error>
        where
            DB: mysql_async::prelude::ToConnection<'a, 't> + 'a,
            T: FromValue,
        {
            Ok(self.pluck(db, 0).await?)
        }

        pub async fn execute<'a, 't: 'a, T, DB>(self, db: DB) -> Result<u64, crate::Error>
        where
            DB: mysql_async::prelude::ToConnection<'a, 't> + 'a,
        {
            Ok(
                mysql_async::prelude::Query::run(self.to_string().with(self.into_args()), db)
                    .await?
                    .affected_rows(),
            )
        }

        pub async fn get_raw<'a, 't: 'a, DB>(
            self,
            db: DB,
        ) -> Result<QueryResult<'a, 't, BinaryProtocol>, crate::Error>
        where
            DB: mysql_async::prelude::ToConnection<'a, 't> + 'a,
        {
            Ok(
                mysql_async::prelude::Query::run(self.to_string().with(self.into_args()), db)
                    .await?,
            )
        }
    }
}
