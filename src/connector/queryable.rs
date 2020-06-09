use super::{ResultSet, Transaction};
use crate::ast::*;
use async_trait::async_trait;

pub trait GetRow {
    fn get_result_row(&self) -> crate::Result<Vec<Value<'static>>>;
}

pub trait TakeRow {
    fn take_result_row(&mut self) -> crate::Result<Vec<Value<'static>>>;
}

pub trait ToColumnNames {
    fn to_column_names(&self) -> Vec<String>;
}

/// Represents a connection or a transaction that can be queried.
#[async_trait]
pub trait Queryable: Send + Sync {
    /// Execute the given query.
    async fn query(&self, q: Query<'_>) -> crate::Result<ResultSet>;

    /// Execute a query given as SQL, interpolating the given parameters.
    async fn query_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet>;

    /// Execute the given query, returning the number of affected rows.
    async fn execute(&self, q: Query<'_>) -> crate::Result<u64>;

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64>;

    /// Run a command in the database, for queries that can't be run using
    /// prepared statements.
    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()>;

    /// Return the version of the underlying database, queried directly from the
    /// source. This corresponds to the `version()` function on PostgreSQL for
    /// example. The version string is returned directly without any form of
    /// parsing or normalization.
    async fn version(&self) -> crate::Result<Option<String>>;

    /// Execute a `SELECT` query.
    async fn select(&self, q: Select<'_>) -> crate::Result<ResultSet> {
        self.query(q.into()).await
    }

    /// Execute an `INSERT` query.
    async fn insert(&self, q: Insert<'_>) -> crate::Result<ResultSet> {
        self.query(q.into()).await
    }

    /// Execute an `UPDATE` query, returning the number of affected rows.
    async fn update(&self, q: Update<'_>) -> crate::Result<u64> {
        self.execute(q.into()).await
    }

    /// Execute a `DELETE` query, returning the number of affected rows.
    async fn delete(&self, q: Delete<'_>) -> crate::Result<()> {
        self.query(q.into()).await?;
        Ok(())
    }

    /// Execute an arbitrary function in the beginning of each transaction.
    async fn server_reset_query(&self, _: &Transaction<'_>) -> crate::Result<()> {
        Ok(())
    }
}

/// A thing that can start a new transaction.
#[async_trait]
pub trait TransactionCapable: Queryable
where
    Self: Sized + Sync,
{
    /// Starts a new transaction
    async fn start_transaction(&self) -> crate::Result<Transaction<'_>> {
        Transaction::new(self).await
    }
}
