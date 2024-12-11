use std::sync::Arc;

use async_trait::async_trait;
use postgres_types::{BorrowToSql, Type};
use tokio_postgres::{Client, Error, RowStream, Statement};

/// Types that can be dispatched to the database as a query and carry the necessary type
/// information about its parameters and columns.
#[async_trait]
pub trait IsQuery: Send {
    fn param_types(&self) -> impl ExactSizeIterator<Item = &Type> + '_;
    fn column_names(&self) -> impl ExactSizeIterator<Item = &str> + '_;
    fn column_types(&self) -> impl ExactSizeIterator<Item = &Type> + '_;

    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send;
}

#[async_trait]
impl IsQuery for Statement {
    fn param_types(&self) -> impl ExactSizeIterator<Item = &Type> + '_ {
        self.params().iter()
    }

    fn column_names(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.columns().iter().map(|c| c.name())
    }

    fn column_types(&self) -> impl ExactSizeIterator<Item = &Type> + '_ {
        self.columns().iter().map(|c| c.type_())
    }

    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send,
    {
        client.query_raw(self, args).await
    }
}

/// A query combined with the type information necessary to run and interpret it.
#[derive(Debug)]
pub struct TypedQuery {
    pub(super) sql: String,
    pub(super) param_types: Vec<Type>,
    pub(super) column_names: Vec<String>,
    pub(super) column_types: Vec<Type>,
}

#[async_trait]
impl IsQuery for TypedQuery {
    fn param_types(&self) -> impl ExactSizeIterator<Item = &Type> + '_ {
        self.param_types.iter()
    }

    fn column_names(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.column_names.iter().map(|s| s.as_str())
    }

    fn column_types(&self) -> impl ExactSizeIterator<Item = &Type> + '_ {
        self.column_types.iter()
    }

    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send,
    {
        client
            .query_typed_raw(&self.sql, args.into_iter().zip(self.param_types.iter().cloned()))
            .await
    }
}

#[async_trait]
impl<A: IsQuery + Sync> IsQuery for Arc<A> {
    #[inline]
    fn param_types(&self) -> impl ExactSizeIterator<Item = &Type> + '_ {
        self.as_ref().param_types()
    }

    #[inline]
    fn column_names(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
        self.as_ref().column_names()
    }

    #[inline]
    fn column_types(&self) -> impl ExactSizeIterator<Item = &Type> + '_ {
        self.as_ref().column_types()
    }

    #[inline]
    async fn dispatch<Args>(&self, client: &Client, args: Args) -> Result<RowStream, Error>
    where
        Args: IntoIterator + Send,
        Args::Item: BorrowToSql,
        Args::IntoIter: ExactSizeIterator + Send,
    {
        self.as_ref().dispatch(client, args).await
    }
}
