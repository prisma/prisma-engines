use crate::{
    ast::{Id, ParameterizedValue, Query},
    error::Error,
    QueryResult,
};

pub trait ToResultRow {
    fn to_result_row<'b>(&'b self) -> QueryResult<ResultRow>;
}

pub trait Transaction {
    fn execute(&mut self, q: Query) -> QueryResult<Option<Id>>;
    fn query(&mut self, q: Query) -> QueryResult<Vec<ResultRow>>;
}

pub trait Transactional {
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

    pub fn as_boolean(&self, i: usize) -> Result<bool, Error> {
        match self.at(i)? {
            ParameterizedValue::Boolean(v) => Ok(*v),
            _ => Err(Error::ResultTypeMissmatch("boolean")),
        }
    }
}
