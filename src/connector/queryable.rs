use super::{ResultSet, Transaction, DBIO};
use crate::ast::*;
use std::ops::Deref;

pub trait ToRow {
    fn to_result_row(&self) -> crate::Result<Vec<ParameterizedValue<'static>>>;
}

pub trait ToColumnNames {
    fn to_column_names(&self) -> Vec<String>;
}

/// Represents a connection or a transaction that can be queried.
pub trait Queryable
where
    Self: Sync,
{
    /// Executes the given query and returns the ID of the last inserted row.
    fn execute<'a>(&'a self, q: Query<'a>) -> DBIO<'a, Option<Id>>;

    /// Executes the given query and returns the result set.
    fn query<'a>(&'a self, q: Query<'a>) -> DBIO<'a, ResultSet>;

    /// Executes a query given as SQL, interpolating the given parameters and
    /// returning a set of results.
    fn query_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [ParameterizedValue<'a>],
    ) -> DBIO<'a, ResultSet>;

    /// Executes a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    fn execute_raw<'a>(&'a self, sql: &'a str, params: &'a [ParameterizedValue]) -> DBIO<'a, u64>;

    /// Turns off all foreign key constraints.
    fn turn_off_fk_constraints(&self) -> DBIO<()>;

    /// Turns on all foreign key constraints.
    fn turn_on_fk_constraints(&self) -> DBIO<()>;

    /// Starts a new transaction
    fn start_transaction(&self) -> DBIO<Transaction>;

    /// Runs a command in the database, for queries that can't be run using
    /// prepared statements.
    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()>;

    /// Empties the given set of tables.
    fn empty_tables<'a>(&'a self, tables: Vec<Table<'a>>) -> DBIO<'a, ()> {
        DBIO::new(async move {
            self.turn_off_fk_constraints().await?;

            for table in tables.into_iter() {
                self.query(Delete::from_table(table).into()).await?;
            }

            self.turn_on_fk_constraints().await?;

            Ok(())
        })
    }

    /// For inserting data. Returns the ID of the last inserted row.
    fn insert<'a>(&'a self, q: Insert<'a>) -> DBIO<'a, Option<Id>> {
        self.execute(q.into())
    }

    /// For updating data.
    fn update<'a>(&'a self, q: Update<'a>) -> DBIO<'a, ()> {
        DBIO::new(async move {
            self.execute(q.into()).await?;
            Ok(())
        })
    }

    /// For deleting data.
    fn delete<'a>(&'a self, q: Delete<'a>) -> DBIO<'a, ()> {
        DBIO::new(async move {
            self.execute(q.into()).await?;
            Ok(())
        })
    }
}

impl<U, T> Queryable for U
where
    U: Deref<Target = T>,
    T: Queryable + 'static,
    Self: Sync,
{
    fn execute<'a>(&'a self, q: Query<'a>) -> DBIO<'a, Option<Id>> {
        T::execute(self, q)
    }

    fn query<'a>(&'a self, q: Query<'a>) -> DBIO<'a, ResultSet> {
        T::query(self, q)
    }

    fn query_raw<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [ParameterizedValue<'a>],
    ) -> DBIO<'a, ResultSet> {
        T::query_raw(self, sql, params)
    }

    fn execute_raw<'a>(&'a self, sql: &'a str, params: &'a [ParameterizedValue]) -> DBIO<'a, u64> {
        T::execute_raw(self, sql, params)
    }

    fn turn_off_fk_constraints(&self) -> DBIO<()> {
        T::turn_off_fk_constraints(self)
    }

    fn turn_on_fk_constraints(&self) -> DBIO<()> {
        T::turn_on_fk_constraints(self)
    }

    fn start_transaction(&self) -> DBIO<Transaction> {
        T::start_transaction(self)
    }

    fn raw_cmd<'a>(&'a self, cmd: &'a str) -> DBIO<'a, ()> {
        T::raw_cmd(self, cmd)
    }
}
