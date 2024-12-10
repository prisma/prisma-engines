use std::sync::Arc;

use async_trait::async_trait;
use postgres_types::{BorrowToSql, Type};
use tokio_postgres::{Client, Error, RowStream, Statement};

#[async_trait]
pub trait IsQuery: Send {
    fn params(&self) -> &[Type];

    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send;
}

#[async_trait]
impl IsQuery for Statement {
    #[inline]
    fn params(&self) -> &[Type] {
        self.params()
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
    pub(super) types: Arc<[Type]>,
}

#[async_trait]
impl IsQuery for TypedQuery {
    #[inline]
    fn params(&self) -> &[Type] {
        &self.types
    }

    #[inline]
    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send,
    {
        client
            .query_typed_raw(&self.sql, args.into_iter().zip(self.types.iter().cloned()))
            .await
    }
}
