use std::{fmt::Display, future::Future, ops::Deref};

use futures_util::{Stream, StreamExt};
use tokio_postgres::{
    row::RowIndex,
    types::{FromSqlOwned, ToSql},
    Client, Row, RowStream,
};

use crate::query::Query;

pub struct PostgresArg(Box<dyn ToSql>);

impl Deref for PostgresArg {
    type Target = dyn ToSql;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<A: ToSql + 'static> From<A> for PostgresArg {
    fn from(value: A) -> Self {
        Self(Box::new(value))
    }
}

type Result<T> = std::result::Result<T, crate::Error>;

impl Query<PostgresArg> {
    pub async fn get<DB, T>(self, db: DB) -> Result<impl IntoIterator<Item = T>>
    where
        DB: Deref<Target = Client>,
        T: From<Row>,
    {
        Ok(self
            .get_raw(db)
            .await?
            .map(|r| Into::<T>::into(r.unwrap()))
            .collect::<Vec<_>>()
            .await)
    }

    pub async fn first<DB, T>(self, db: DB) -> Result<Option<T>>
    where
        DB: Deref<Target = Client>,
        T: From<Row>,
    {
        Ok(self.get(db).await?.into_iter().nth(0))
    }

    pub async fn pluck<DB, I, T>(self, db: DB, idx: I) -> Result<impl Stream<Item = T>>
    where
        DB: Deref<Target = Client>,
        I: RowIndex + Display + Clone,
        T: FromSqlOwned,
    {
        Ok(self
            .get_raw(db)
            .await?
            .map(move |r| r.unwrap().get::<I, T>(idx.clone())))
    }

    pub async fn all_values<DB, T>(self, db: DB) -> Result<impl Stream<Item = T>>
    where
        DB: Deref<Target = Client>,
        T: FromSqlOwned,
    {
        Ok(self.pluck(db, 0).await?)
    }

    pub async fn value<DB, T>(self, db: DB) -> Result<Option<T>>
    where
        DB: Deref<Target = Client>,
        T: FromSqlOwned,
    {
        let values = self.all_values(db).await?;
        futures_util::pin_mut!(values);

        Ok(values.into_future().await.0)
    }

    pub async fn execute<DB, T>(self, db: DB) -> Result<u64>
    where
        DB: Deref<Target = Client>,
    {
        let (query, args) = self.build()?;

        Ok(db
            .deref()
            .execute_raw(&query, args.iter().map(Deref::deref).collect::<Vec<_>>())
            .await?)
    }

    pub async fn get_raw<DB>(self, db: DB) -> Result<RowStream>
    where
        DB: Deref<Target = Client>,
    {
        let (query, args) = self.build()?;

        Ok(db
            .deref()
            .query_raw(&query, args.iter().map(Deref::deref).collect::<Vec<_>>())
            .await?)
    }
}

pub trait ClientExt: Deref<Target = Client> {
    fn get<T>(
        &self,
        query: Query<PostgresArg>,
    ) -> impl Future<Output = Result<impl IntoIterator<Item = T>>>
    where
        T: From<Row>,
    {
        query.get(self.deref())
    }

    fn get_raw(&self, query: Query<PostgresArg>) -> impl Future<Output = Result<RowStream>> {
        query.get_raw(self.deref())
    }
}

impl ClientExt for &Client {}
