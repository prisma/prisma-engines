use crate::{
    ast::ParameterizedValue,
    error::Error,
    transaction::{ColumnNames, ResultRow},
};
use std::{collections::HashMap, ops, rc::Rc};

/// Encapsulates a set of results and their respective column names.
#[derive(Debug)]
pub struct ResultSet {
    /// DO NOT expose these fields.
    /// ResultSet might become lazy-loading one day,
    /// no longer backed by a vector.
    pub(crate) rows: Vec<ResultRow>,
    pub(crate) name_to_index: HashMap<String, usize>,
}

impl ResultSet {
    /// Creates a new instance, bound to the given column names and result rows.
    pub fn new(names: &ColumnNames, values: Vec<ResultRow>) -> ResultSet {
        ResultSet {
            name_to_index: Self::build_name_map(names),
            rows: values,
        }
    }

    /// Creates a lookup map for column names.
    fn build_name_map(names: &ColumnNames) -> HashMap<String, usize> {
        let mut mapped = HashMap::<String, usize>::new();

        for i in 0..names.names.len() {
            mapped.insert(names.names[i].clone(), i);
        }

        mapped
    }
}

impl IntoIterator for ResultSet {
    type Item = ResultRowWithName;
    type IntoIter = ResultSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        ResultSetIterator {
            name_to_index: Rc::new(self.name_to_index),
            internal_iterator: self.rows.into_iter(),
        }
    }
}

/// Thin iterator for ResultSet rows.
/// Might become lazy one day.
pub struct ResultSetIterator {
    name_to_index: Rc<HashMap<String, usize>>,
    internal_iterator: std::vec::IntoIter<ResultRow>,
}

impl Iterator for ResultSetIterator {
    type Item = ResultRowWithName;

    fn next(&mut self) -> Option<Self::Item> {
        match self.internal_iterator.next() {
            Some(row) => Some(ResultRowWithName {
                name_to_index: Rc::clone(&self.name_to_index),
                values: row,
            }),
            None => None,
        }
    }
}

/// Wraps a result row, so it's columns can be accessed
/// by name.
pub struct ResultRowWithName {
    name_to_index: Rc<HashMap<String, usize>>,
    values: ResultRow,
}

impl ops::Index<usize> for ResultRowWithName {
    type Output = ParameterizedValue<'static>;

    fn index(&self, index: usize) -> &ParameterizedValue<'static> {
        &self.values.values[index]
    }
}

impl ResultRowWithName {
    // TODO: If the API is fixed, reduce internal duplication by moving
    // getters for specific types to ParameterizedValue

    // Index Getters

    /// Gets a value by index.
    pub fn at(&self, i: usize) -> Result<&ParameterizedValue, Error> {
        if self.values.values.len() <= i {
            Err(Error::ResultIndexOutOfBounds(i))
        } else {
            Ok(&self.values.values[i])
        }
    }

    pub fn at_as_str(&self, i: usize) -> Result<&str, Error> {
        match self.at(i)? {
            ParameterizedValue::Text(s) => Ok(&*s),
            _ => Err(Error::ResultTypeMismatch("string")),
        }
    }

    pub fn at_as_string(&self, i: usize) -> Result<String, Error> {
        match self.at(i)? {
            ParameterizedValue::Text(s) => Ok(s.clone().into_owned()),
            _ => Err(Error::ResultTypeMismatch("string")),
        }
    }

    pub fn at_as_integer(&self, i: usize) -> Result<i64, Error> {
        match self.at(i)? {
            ParameterizedValue::Integer(v) => Ok(*v),
            _ => Err(Error::ResultTypeMismatch("integer")),
        }
    }

    pub fn at_as_real(&self, i: usize) -> Result<f64, Error> {
        match self.at(i)? {
            ParameterizedValue::Real(v) => Ok(*v),
            _ => Err(Error::ResultTypeMismatch("real")),
        }
    }

    pub fn at_as_bool(&self, i: usize) -> Result<bool, Error> {
        match self.at(i)? {
            ParameterizedValue::Boolean(v) => Ok(*v),
            ParameterizedValue::Integer(v) => Ok(*v != 0),
            _ => Err(Error::ResultTypeMismatch("boolean")),
        }
    }

    // Name Getters

    /// Gets a value by column name.
    pub fn get(&self, name: &str) -> Result<&ParameterizedValue, Error> {
        match self.name_to_index.get(name) {
            None => Err(Error::ColumnNotFound(String::from(name))),
            Some(idx) => Ok(&self.values.values[*idx]),
        }
    }

    pub fn get_as_str(&self, name: &str) -> Result<&str, Error> {
        match self.get(name)? {
            ParameterizedValue::Text(s) => Ok(&*s),
            _ => Err(Error::ResultTypeMismatch("string")),
        }
    }

    pub fn get_as_string(&self, name: &str) -> Result<String, Error> {
        match self.get(name)? {
            ParameterizedValue::Text(s) => Ok(s.clone().into_owned()),
            _ => Err(Error::ResultTypeMismatch("string")),
        }
    }

    pub fn get_as_integer(&self, name: &str) -> Result<i64, Error> {
        match self.get(name)? {
            ParameterizedValue::Integer(v) => Ok(*v),
            _ => Err(Error::ResultTypeMismatch("integer")),
        }
    }

    pub fn get_as_real(&self, name: &str) -> Result<f64, Error> {
        match self.get(name)? {
            ParameterizedValue::Real(v) => Ok(*v),
            _ => Err(Error::ResultTypeMismatch("real")),
        }
    }

    pub fn get_as_bool(&self, name: &str) -> Result<bool, Error> {
        match self.get(name)? {
            ParameterizedValue::Boolean(v) => Ok(*v),
            ParameterizedValue::Integer(v) => Ok(*v != 0),
            _ => Err(Error::ResultTypeMismatch("boolean")),
        }
    }
}
