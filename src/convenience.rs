use std::collections::HashMap;
use crate::{error::Error, transaction::{ ResultRow, ColumnNames }, ast::ParameterizedValue};

/// Encapsulates a set of results and their respective column names.
pub struct ResultSet<'a> {
    values: &'a Vec<ResultRow>,
    #[allow(dead_code)]
    names: &'a ColumnNames,
    mapped_names: HashMap<&'a str, usize>
}

/// Convenience conversion method.
impl<'a> From<(&'a ColumnNames, &'a Vec<ResultRow>)> for ResultSet<'a> {
    fn from(result: (&'a ColumnNames, &'a Vec<ResultRow>)) -> ResultSet<'a>  {
        ResultSet::<'a>::new(result.0, result.1)
    }
}

impl<'a> ResultSet<'a> {
    /// Creates a new instance, bound to the given column names and result rows.
    pub fn new(names: &'a ColumnNames, values: &'a Vec<ResultRow>) -> ResultSet<'a> {
        let mut mapped = HashMap::<&'a str, usize>::new();

        for (i, name) in names.names.iter().enumerate() {
            mapped.insert(name, i);
        }

        ResultSet {
            mapped_names: mapped,
            values: values,
            names: names
        }
    }

    /// Finds a column index for a name.
    pub fn column_index(&self, name: &str) -> Result<usize, Error> {
        match self.mapped_names.get(name) {
            None => Err(Error::ColumnNotFound(String::from(name))),
            Some(idx) => Ok(*idx)
        }
    }

    /// Returns an interator over wrapped result rows.
    pub fn iter(&'a self) -> impl std::iter::Iterator<Item = ResultRowWithName<'a>> {
        self.values.iter().map(move |val| ResultRowWithName::<'a> { values: val, parent_set: self })
    }
}

/// Wraps a result row, so it's columns can be accessed
/// by name.
pub struct ResultRowWithName<'a> {
    parent_set: &'a ResultSet<'a>,
    values: &'a ResultRow
}

impl<'a>  ResultRowWithName<'a>  {

    // TODO: If the API is fixed, reduce internal duplication by moving
    // getters for specific types to ParameterizedValue

    // Index Getters

    /// Gets a value by index.
    pub fn at(&self, i: usize) -> Result<&ParameterizedValue, Error> {
        if self.values.values.len() <= i {
            Err(Error::ResultIndexOutOfBounts(i))
        } else {
            Ok(&self.values.values[i])
        }
    }

    pub fn at_as_str(&self, i: usize) -> Result<&str, Error> {
        match self.at(i)? {
            ParameterizedValue::Text(s) => Ok(s),
            _ => Err(Error::ResultTypeMissmatch("string")),
        }
    }

    pub fn at_as_string(&self, i: usize) -> Result<String, Error> {
        match self.at(i)? {
            ParameterizedValue::Text(s) => Ok(s.clone()),
            _ => Err(Error::ResultTypeMissmatch("string")),
        }
    }

    pub fn at_as_integer(&self, i: usize) -> Result<i64, Error> {
        match self.at(i)? {
            ParameterizedValue::Integer(v) => Ok(*v),
            _ => Err(Error::ResultTypeMissmatch("integer")),
        }
    }

    pub fn at_as_real(&self, i: usize) -> Result<f64, Error> {
        match self.at(i)? {
            ParameterizedValue::Real(v) => Ok(*v),
            _ => Err(Error::ResultTypeMissmatch("real")),
        }
    }

    pub fn at_as_bool(&self, i: usize) -> Result<bool, Error> {
        match self.at(i)? {
            ParameterizedValue::Boolean(v) => Ok(*v),
            ParameterizedValue::Integer(v) => Ok(*v != 0),
            _ => Err(Error::ResultTypeMissmatch("boolean")),
        }
    }

    // Name Getters

    /// Gets a value by column name.
    pub fn get(&self, name: &str) -> Result<&ParameterizedValue, Error> {
        let idx = self.parent_set.column_index(name)?;
        Ok(&self.values.values[idx])
    }

    pub fn get_as_str(&self, name: &str) -> Result<&str, Error> {
        match self.get(name)? {
            ParameterizedValue::Text(s) => Ok(s),
            _ => Err(Error::ResultTypeMissmatch("string")),
        }
    }

    pub fn get_as_string(&self, name: &str) -> Result<String, Error> {
        match self.get(name)? {
            ParameterizedValue::Text(s) => Ok(s.clone()),
            _ => Err(Error::ResultTypeMissmatch("string")),
        }
    }

    pub fn get_as_integer(&self, name: &str) -> Result<i64, Error> {
        match self.get(name)? {
            ParameterizedValue::Integer(v) => Ok(*v),
            _ => Err(Error::ResultTypeMissmatch("integer")),
        }
    }

    pub fn get_as_real(&self, name: &str) -> Result<f64, Error> {
        match self.get(name)? {
            ParameterizedValue::Real(v) => Ok(*v),
            _ => Err(Error::ResultTypeMissmatch("real")),
        }
    }

    pub fn get_as_bool(&self, name: &str) -> Result<bool, Error> {
        match self.get(name)? {
            ParameterizedValue::Boolean(v) => Ok(*v),
            ParameterizedValue::Integer(v) => Ok(*v != 0),
            _ => Err(Error::ResultTypeMissmatch("boolean")),
        }
    }

}
