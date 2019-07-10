use crate::{
    ast::{Id, ParameterizedValue, Query},
    ResultSet,
};

pub trait ToRow {
    fn to_result_row<'b>(&'b self) -> crate::Result<Row>;
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
    type Error;

    /// Opens a connection and a transaction, which is valid inside the given handler closure.
    ///
    /// The transaction is comitted if the result returned by the handler is Ok.
    /// Otherise, the transaction is discarded.
    fn with_transaction<F, T>(&self, db: &str, f: F) -> std::result::Result<T, Self::Error>
    where
        F: FnOnce(&mut Transaction) -> std::result::Result<T, Self::Error>;
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Row {
    pub values: Vec<ParameterizedValue<'static>>,
}

impl<T> From<Vec<T>> for Row
where
    T: Into<ParameterizedValue<'static>>,
{
    fn from(values: Vec<T>) -> Self {
        Self {
            values: values.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ColumnNames {
    pub names: Vec<String>,
}

impl<T> From<Vec<T>> for ColumnNames
where
    T: Into<String>,
{
    fn from(names: Vec<T>) -> Self {
        Self {
            names: names.into_iter().map(Into::into).collect(),
        }
    }
}
