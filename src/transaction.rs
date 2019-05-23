use crate::{
    ast::{Id, ParameterizedValue, Query},
    error::Error,
    QueryResult,
};

pub trait ToResultRow {
    fn to_result_row<'b>(&'b self) -> QueryResult<ResultRow>;
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
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>>;

    /// Executes the given query and returns the result set.
    ///
    /// This is typically used for select queries.
    fn query(&mut self, q: Query) -> QueryResult<Vec<ResultRow>>;

    /// Executes a query given as SQL, interpolating the given parameters.
    ///
    /// This is needed, for example, for PRAGMA commands in sqlite.
    fn query_raw(
        &mut self,
        sql: &str,
        params: &[ParameterizedValue],
    ) -> QueryResult<Vec<ResultRow>>;
}

pub trait Connectional {
    /// Opens a connection, which is valid inside the given handler closure..
    ///
    /// This method does not open a transaction, and should used for
    /// operations not requiring transactions, e.g. single queries
    /// or schema mutations.
    fn with_connection<F, T>(&self, db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut Connection) -> QueryResult<T>;
}

pub trait Transactional {
    /// Opens a connection and a transaction, which is valid inside the given handler closure.
    ///
    /// The transaction is comitted if the result returned by the handler is Ok.
    /// Otherise, the transaction is discarded.
    fn with_transaction<F, T>(&self, db: &str, f: F) -> QueryResult<T>
    where
        F: FnOnce(&mut Transaction) -> QueryResult<T>;
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct ResultRow {
    pub values: Vec<ParameterizedValue>,
}

impl ResultRow {
    pub fn at(&self, i: usize) -> Result<&ParameterizedValue, Error> {
        if self.values.len() <= i {
            Err(Error::ResultIndexOutOfBounts(i))
        } else {
            Ok(&self.values[i])
        }
    }

    pub fn as_str(&self, i: usize) -> Result<&str, Error> {
        match self.at(i)? {
            ParameterizedValue::Text(s) => Ok(s),
            _ => Err(Error::ResultTypeMissmatch("string")),
        }
    }

    pub fn as_string(&self, i: usize) -> Result<String, Error> {
        match self.at(i)? {
            ParameterizedValue::Text(s) => Ok(s.clone()),
            _ => Err(Error::ResultTypeMissmatch("string")),
        }
    }

    pub fn as_integer(&self, i: usize) -> Result<i64, Error> {
        match self.at(i)? {
            ParameterizedValue::Integer(v) => Ok(*v),
            _ => Err(Error::ResultTypeMissmatch("integer")),
        }
    }

    pub fn as_real(&self, i: usize) -> Result<f64, Error> {
        match self.at(i)? {
            ParameterizedValue::Real(v) => Ok(*v),
            _ => Err(Error::ResultTypeMissmatch("real")),
        }
    }

    pub fn as_bool(&self, i: usize) -> Result<bool, Error> {
        match self.at(i)? {
            ParameterizedValue::Boolean(v) => Ok(*v),
            _ => Err(Error::ResultTypeMissmatch("boolean")),
        }
    }
}
