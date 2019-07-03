use crate::{
    ast::{Id, ParameterizedValue, Query},
    ResultSet,
};

pub trait ToResultRow {
    fn to_result_row<'b>(&'b self) -> crate::Result<ResultRow>;
}

pub trait ToColumnNames {
    fn to_column_names<'b>(&'b self) -> ColumnNames;
}

/// Represents a transaction.
pub trait Transaction: Connection {}

// Note: The methods here have somewhat cumbersome
// naming, so they do not clash with names exported from
// rusqlite etc.

/// Represents a connection.
pub trait Connection {
    /// Executes the given query and returns the ID of the last
    /// inserted row.
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
    /// Opens a connection and a transaction, which is valid inside the given handler closure.
    ///
    /// The transaction is comitted if the result returned by the handler is Ok.
    /// Otherise, the transaction is discarded.
    fn with_transaction<F, T>(&self, db: &str, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Transaction) -> crate::Result<T>;
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ResultRow {
    pub values: Vec<ParameterizedValue<'static>>,
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ColumnNames {
    pub names: Vec<String>,
}
