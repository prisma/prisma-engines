use super::{IsolationLevel, ResultSet, Transaction};
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

    /// Execute a query given as SQL, interpolating the given parameters.
    ///
    /// On Postgres, query parameters types will be inferred from the values
    /// instead of letting Postgres infer them based on their usage in the SQL query.
    ///
    /// NOTE: This method will eventually be removed & merged into Queryable::query_raw().
    async fn query_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<ResultSet>;

    /// Execute the given query, returning the number of affected rows.
    async fn execute(&self, q: Query<'_>) -> crate::Result<u64>;

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    async fn execute_raw(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64>;

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    ///
    /// On Postgres, query parameters types will be inferred from the values
    /// instead of letting Postgres infer them based on their usage in the SQL query.
    ///
    /// NOTE: This method will eventually be removed & merged into Queryable::query_raw().
    async fn execute_raw_typed(&self, sql: &str, params: &[Value<'_>]) -> crate::Result<u64>;

    /// Run a command in the database, for queries that can't be run using
    /// prepared statements.
    async fn raw_cmd(&self, cmd: &str) -> crate::Result<()>;

    /// Return the version of the underlying database, queried directly from the
    /// source. This corresponds to the `version()` function on PostgreSQL for
    /// example. The version string is returned directly without any form of
    /// parsing or normalization.
    async fn version(&self) -> crate::Result<Option<String>>;

    /// Returns false, if connection is considered to not be in a working state.
    fn is_healthy(&self) -> bool;

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
    async fn server_reset_query(&self, _: &dyn Transaction) -> crate::Result<()> {
        Ok(())
    }

    /// Statement to begin a transaction
    async fn begin_statement(&self, depth: i32) -> String {
        let savepoint_stmt = format!("SAVEPOINT savepoint{}", depth);
        let ret = if depth > 1 { savepoint_stmt } else { "BEGIN".to_string() };

        return ret;
    }

    /// Statement to commit a transaction
    async fn commit_statement(&self, depth: i32) -> String {
        let savepoint_stmt = format!("RELEASE SAVEPOINT savepoint{}", depth);
        let ret = if depth > 1 {
            savepoint_stmt
        } else {
            "COMMIT".to_string()
        };

        return ret;
    }

    /// Statement to rollback a transaction
    async fn rollback_statement(&self, depth: i32) -> String {
        let savepoint_stmt = format!("ROLLBACK TO SAVEPOINT savepoint{}", depth);
        let ret = if depth > 1 {
            savepoint_stmt
        } else {
            "ROLLBACK".to_string()
        };

        return ret;
    }

    /// Sets the transaction isolation level to given value.
    /// Implementers have to make sure that the passed isolation level is valid for the underlying database.
    async fn set_tx_isolation_level(&self, isolation_level: IsolationLevel) -> crate::Result<()>;

    /// Signals if the isolation level SET needs to happen before or after the tx BEGIN.
    fn requires_isolation_first(&self) -> bool;
}

/// A thing that can start a new transaction.
#[async_trait]
pub trait TransactionCapable: Queryable {
    /// Starts a new transaction
    async fn start_transaction<'a>(
        &'a self,
        isolation: Option<IsolationLevel>,
    ) -> crate::Result<Box<dyn Transaction + 'a>>;
}

macro_rules! impl_default_TransactionCapable {
    ($t:ty) => {
        #[async_trait]
        impl TransactionCapable for $t {
            async fn start_transaction<'a>(
                &'a self,
                isolation: Option<IsolationLevel>,
            ) -> crate::Result<Box<dyn crate::connector::Transaction + 'a>> {
                let opts = crate::connector::TransactionOptions::new(
                    isolation,
                    self.requires_isolation_first(),
                    self.transaction_depth.clone(),
                );

                Ok(Box::new(
                    crate::connector::DefaultTransaction::new(self, opts).await?,
                ))
            }
        }
    };
}

pub(crate) use impl_default_TransactionCapable;
