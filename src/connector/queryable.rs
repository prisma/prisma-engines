use super::{ResultSet, Transaction, DBIO};
use crate::ast::*;

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
pub trait Queryable
where
    Self: Sync,
{
    /// Execute the given query.
    fn query<'a>(&'a self, q: Query<'a>) -> DBIO<'a, ResultSet>;

    /// Execute a query given as SQL, interpolating the given parameters.
    fn query_raw<'a>(&'a self, sql: &'a str, params: &'a [Value<'a>]) -> DBIO<'a, ResultSet>;

    /// Execute the given query, returning the number of affected rows.
    fn execute<'a>(&'a self, q: Query<'a>) -> DBIO<'a, u64>;

    /// Execute a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    fn execute_raw<'a>(&'a self, sql: &'a str, params: &'a [Value<'a>]) -> DBIO<'a, u64>;

    /// Run a command in the database, for queries that can't be run using
    /// prepared statements.
    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()>;

    /// Execute a `SELECT` query.
    fn select<'a>(&'a self, q: Select<'a>) -> DBIO<'a, ResultSet> {
        self.query(q.into())
    }

    /// Execute an `INSERT` query.
    fn insert<'a>(&'a self, q: Insert<'a>) -> DBIO<'a, ResultSet> {
        self.query(q.into())
    }

    /// Execute an `UPDATE` query, returning the number of affected rows.
    fn update<'a>(&'a self, q: Update<'a>) -> DBIO<'a, u64> {
        self.execute(q.into())
    }

    /// Execute a `DELETE` query, returning the number of affected rows.
    fn delete<'a>(&'a self, q: Delete<'a>) -> DBIO<'a, ()> {
        DBIO::new(async move {
            self.query(q.into()).await?;
            Ok(())
        })
    }

    /// Return the version of the underlying database, queried directly from the source. This
    /// corresponds to the `version()` function on PostgreSQL for example. The version string is
    /// returned directly without any form of parsing or normalization.
    fn version<'a>(&'a self) -> DBIO<'a, Option<String>>;

    /// Execute an arbitrary function in the beginning of each transaction.
    fn server_reset_query<'a>(&'a self, _: &'a Transaction<'a>) -> DBIO<'a, ()> {
        DBIO::new(futures::future::ready(Ok(())))
    }
}

/// A thing that can start a new transaction.
pub trait TransactionCapable: Queryable
where
    Self: Sized + Sync,
{
    /// Starts a new transaction
    fn start_transaction(&self) -> DBIO<Transaction> {
        DBIO::new(async move { Ok(Transaction::new(self).await?) })
    }
}
