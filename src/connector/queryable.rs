use super::{ResultSet, Transaction};
use crate::ast::*;

pub trait ToRow {
    fn to_result_row<'b>(&'b self) -> crate::Result<Vec<ParameterizedValue<'static>>>;
}

pub trait ToColumnNames {
    fn to_column_names<'b>(&'b self) -> Vec<String>;
}

/// Represents a connection.
pub trait Queryable
{
    /// Executes the given query and returns the ID of the last inserted row.
    ///
    /// This is typically used for mutating queries.
    fn execute(&mut self, q: Query) -> crate::Result<Option<Id>>;

    /// Executes the given query and returns the result set.
    ///
    /// This is typically used for select queries.
    fn query(&mut self, q: Query) -> crate::Result<ResultSet>;

    /// Executes a query given as SQL, interpolating the given parameters and
    /// returning a set of results.
    fn query_raw(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue],
    ) -> crate::Result<ResultSet>;

    /// Executes a query given as SQL, interpolating the given parameters and
    /// returning the number of affected rows.
    fn execute_raw(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue],
    ) -> crate::Result<u64>;

    /// Turns off all foreign key constraints.
    fn turn_off_fk_constraints(&mut self) -> crate::Result<()>;

    /// Turns on all foreign key constraints.
    fn turn_on_fk_constraints(&mut self) -> crate::Result<()>;

    /// Empties the given set of tables.
    fn empty_tables(&mut self, tables: Vec<Table>) -> crate::Result<()> {
        self.turn_off_fk_constraints()?;

        for table in tables {
            self.query(Delete::from_table(table).into())?;
        }

        self.turn_on_fk_constraints()?;

        Ok(())
    }

    /// Starts a new transaction
    fn start_transaction<'a>(&'a mut self) -> crate::Result<Transaction<'a>>;

    /// Runs a command in the database, for queries that can't be run using
    /// prepared statements.
    fn raw_cmd<'a>(&mut self, cmd: &str) -> crate::Result<()>;

    fn insert(&mut self, q: Insert) -> crate::Result<Option<Id>> {
        self.execute(q.into())
    }

    fn update(&mut self, q: Update) -> crate::Result<()> {
        self.execute(q.into())?;
        Ok(())
    }

    fn delete(&mut self, q: Delete) -> crate::Result<()> {
        self.execute(q.into())?;
        Ok(())
    }
}
