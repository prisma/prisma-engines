use std::sync::Arc;

use async_trait::async_trait;
use postgres_types::{BorrowToSql, Type};
use tokio_postgres::{Client, Column, Error, RowStream, Statement};

#[async_trait]
pub trait IsQuery: Send {
    fn params(&self) -> impl ExactSizeIterator<Item = Type> + '_;
    fn columns(&self) -> impl ExactSizeIterator<Item = Type> + '_;

    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send;
}

#[async_trait]
impl IsQuery for Statement {
    fn params(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.params().iter().cloned()
    }

    fn columns(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.columns().iter().map(Column::type_).cloned()
    }

    #[inline]
    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send,
    {
        client.query_raw(self, args).await
    }
}

#[derive(Debug, Clone)]
pub struct TypedQuery {
    pub(super) sql: Arc<str>,
    pub(super) params: Arc<[Type]>,
    pub(super) columns: Arc<[Type]>,
}

#[async_trait]
impl IsQuery for TypedQuery {
    fn params(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.params.iter().cloned()
    }

    fn columns(&self) -> impl ExactSizeIterator<Item = Type> + '_ {
        self.columns.iter().cloned()
    }

    #[inline]
    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send,
    {
        client
            .query_typed_raw(&self.sql, args.into_iter().zip(self.params.iter().cloned()))
            .await
    }
}
