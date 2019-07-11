use super::ResultSet;
use crate::ast::*;

pub trait ToRow {
    fn to_result_row<'b>(&'b self) -> crate::Result<Vec<ParameterizedValue<'static>>>;
}

pub trait ToColumnNames {
    fn to_column_names<'b>(&'b self) -> Vec<String>;
}

/// Represents a transaction.
pub trait Transaction: Connection {}

/// Represents a connection.
pub trait Connection {
    /// Executes the given query and returns the ID of the last inserted row.
    ///
    /// This is typically used for mutating queries.
    fn execute<'a>(&mut self, q: Query<'a>) -> crate::Result<Option<Id>>;

    /// Executes the given query and returns the result set.
    ///
    /// This is typically used for select queries.
    fn query<'a>(&mut self, q: Query<'a>) -> crate::Result<ResultSet>;

    /// Executes a query given as SQL, interpolating the given parameters.
    ///
    /// This is needed, for example, for PRAGMA commands in sqlite.
    fn query_raw<'a>(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet>;

    /// Turns off all foreign key constraints.
    fn turn_off_fk_constraints(&mut self) -> crate::Result<()>;

    /// Turns on all foreign key constraints.
    fn turn_on_fk_constraints(&mut self) -> crate::Result<()>;

    /// Empties the given set of tables.
    fn empty_tables<'a>(&mut self, tables: Vec<Table>) -> crate::Result<()> {
        self.turn_off_fk_constraints()?;

        for table in tables {
            self.query(Delete::from_table(table).into())?;
        }

        self.turn_on_fk_constraints()?;

        Ok(())
    }
}

pub trait Connectional {
    /// Opens a connection, which is valid inside the given handler closure..
    ///
    /// This method does not open a transaction, and should used for
    /// operations not requiring transactions, e.g. single queries
    /// or schema mutations.
    fn with_connection<F, T>(&self, db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Connection) -> crate::Result<T>,
        Self: Sized;

    fn execute_on_connection<'a>(&self, db: &str, query: Query<'a>) -> crate::Result<Option<Id>>;

    fn query_on_connection<'a>(&self, db: &str, query: Query<'a>) -> crate::Result<ResultSet>;

    fn query_on_raw_connection<'a>(
        &self,
        db: &str,
        sql: &str,
        params: &[ParameterizedValue<'a>],
    ) -> crate::Result<ResultSet>;
}

pub trait Transactional {
    type Error;

    /// Opens a connection and a transaction, which is valid inside the given handler closure.
    ///
    /// The transaction is comitted if the result returned by the handler is Ok.
    /// Otherise, the transaction is discarded.
    fn with_transaction<F, T>(&self, db: &str, f: F) -> std::result::Result<T, Self::Error>
    where
        F: FnOnce(&mut Transaction) -> std::result::Result<T, Self::Error>;
}
