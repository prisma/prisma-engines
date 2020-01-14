use super::{ResultSet, Transaction, DBIO};
use crate::ast::*;

pub trait GetRow {
    fn get_result_row(&self) -> crate::Result<Vec<ParameterizedValue<'static>>>;
}

pub trait TakeRow {
    fn take_result_row(&mut self) -> crate::Result<Vec<ParameterizedValue<'static>>>;
}

pub trait ToColumnNames {
    fn to_column_names(&self) -> Vec<String>;
}

/// Represents a connection or a transaction that can be queried.
pub trait Queryable
where
    Self: Sync,
{
    /// Executes the given query and returns the result set.
    fn query<'a>(&'a self, q: Query<'a>) -> DBIO<'a, ResultSet>;

    /// Executes a query given as SQL, interpolating the given parameters and
    /// returning a set of results.
    fn query_raw<'a>(&'a self, sql: &'a str, params: &'a [ParameterizedValue<'a>]) -> DBIO<'a, ResultSet>;

    /// Runs a command in the database, for queries that can't be run using
    /// prepared statements.
    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()>;

    // For selecting data returning the results.
    fn select<'a>(&'a self, q: Select<'a>) -> DBIO<'a, ResultSet> {
        self.query(q.into())
    }

    /// For inserting data. Returns the ID of the last inserted row.
    fn insert<'a>(&'a self, q: Insert<'a>) -> DBIO<'a, ResultSet> {
        self.query(q.into())
    }

    /// For updating data.
    fn update<'a>(&'a self, q: Update<'a>) -> DBIO<'a, ()> {
        DBIO::new(async move {
            self.query(q.into()).await?;
            Ok(())
        })
    }

    /// For deleting data.
    fn delete<'a>(&'a self, q: Delete<'a>) -> DBIO<'a, ()> {
        DBIO::new(async move {
            self.query(q.into()).await?;
            Ok(())
        })
    }
}

/// A thing that can start a new transaction.
pub trait TransactionCapable: Queryable
where
    Self: Sized + Sync,
{
    /// Starts a new transaction
    fn start_transaction(&self) -> DBIO<Transaction> {
        DBIO::new(async move { Transaction::new(self).await })
    }
}
