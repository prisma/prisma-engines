use super::{ResultSet, Transaction};
use crate::ast::*;
use std::ops::DerefMut;

pub trait ToRow {
    fn to_result_row(&self) -> crate::Result<Vec<ParameterizedValue<'static>>>;
}

pub trait ToColumnNames {
    fn to_column_names(&self) -> Vec<String>;
}

/// Represents a connection or a transaction that can be queried.
pub trait Queryable {
    /// Executes the given query and returns the ID of the last inserted row.
    fn execute(&mut self, q: Query) -> crate::Result<Option<Id>>;

    /// Executes the given query and returns the result set.
    fn query(&mut self, q: Query) -> crate::Result<ResultSet>;

    /// Executes a query given as SQL, interpolating the given parameters and
    /// returning a set of results.
    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> crate::Result<ResultSet>;

    /// Executes a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    fn execute_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> crate::Result<u64>;

    /// Turns off all foreign key constraints.
    fn turn_off_fk_constraints(&mut self) -> crate::Result<()>;

    /// Turns on all foreign key constraints.
    fn turn_on_fk_constraints(&mut self) -> crate::Result<()>;

    /// Starts a new transaction
    fn start_transaction<'a>(&'a mut self) -> crate::Result<Transaction<'a>>;

    /// Runs a command in the database, for queries that can't be run using
    /// prepared statements.
    fn raw_cmd(&mut self, cmd: &str) -> crate::Result<()>;

    /// Empties the given set of tables.
    fn empty_tables(&mut self, tables: Vec<Table>) -> crate::Result<()> {
        self.turn_off_fk_constraints()?;

        for table in tables {
            self.query(Delete::from_table(table).into())?;
        }

        self.turn_on_fk_constraints()?;

        Ok(())
    }

    /// For inserting data. Returns the ID of the last inserted row.
    fn insert(&mut self, q: Insert) -> crate::Result<Option<Id>> {
        self.execute(q.into())
    }

    /// For updating data.
    fn update(&mut self, q: Update) -> crate::Result<()> {
        self.execute(q.into())?;
        Ok(())
    }

    /// For deleting data.
    fn delete(&mut self, q: Delete) -> crate::Result<()> {
        self.execute(q.into())?;
        Ok(())
    }
}

impl<Q: Queryable> Queryable for dyn DerefMut<Target = Q> {
    fn execute(&mut self, q: Query) -> crate::Result<Option<Id>> {
        self.deref_mut().execute(q)
    }

    fn query(&mut self, q: Query) -> crate::Result<ResultSet> {
        self.deref_mut().query(q)
    }

    fn query_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> crate::Result<ResultSet> {
        self.deref_mut().query_raw(sql, params)
    }

    fn execute_raw(&mut self, sql: &str, params: &[ParameterizedValue]) -> crate::Result<u64> {
        self.deref_mut().execute_raw(sql, params)
    }

    fn turn_off_fk_constraints(&mut self) -> crate::Result<()> {
        self.deref_mut().turn_off_fk_constraints()
    }

    fn turn_on_fk_constraints(&mut self) -> crate::Result<()> {
        self.deref_mut().turn_on_fk_constraints()
    }

    fn start_transaction<'a>(&'a mut self) -> crate::Result<Transaction<'a>> {
        self.deref_mut().start_transaction()
    }

    fn raw_cmd(&mut self, cmd: &str) -> crate::Result<()> {
        self.deref_mut().raw_cmd(cmd)
    }
}
