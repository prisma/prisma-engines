use crate::{
    ast::Value,
    connector::ColumnType,
    error::{Error, ErrorKind},
};
use std::sync::Arc;

/// An owned version of a `Row` in a `ResultSet`. See
/// [ResultRowRef](struct.ResultRowRef.html) for documentation on data access.
#[derive(Debug, PartialEq)]
pub struct ResultRow {
    pub(crate) columns: Arc<Vec<String>>,
    pub(crate) types: Vec<ColumnType>,
    pub(crate) values: Vec<Value<'static>>,
}

impl IntoIterator for ResultRow {
    type Item = Value<'static>;
    type IntoIter = std::vec::IntoIter<Value<'static>>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

/// A reference to a `Row` in a `ResultSet`. The columns can be accessed either
/// through their position or using the column name.
///
/// ```
/// # use quaint::connector::*;
/// let names = vec!["id".to_string(), "name".to_string()];
/// let rows = vec![vec!["1234".into(), "Musti".into()]];
///
/// let result_set = ResultSet::new(names, rows);
/// let row = result_set.first().unwrap();
///
/// assert_eq!(row[0], row["id"]);
/// assert_eq!(row[1], row["name"]);
/// ```
#[derive(Debug, PartialEq)]
pub struct ResultRowRef<'a> {
    pub(crate) columns: Arc<Vec<String>>,
    pub(crate) types: Vec<ColumnType>,
    pub(crate) values: &'a Vec<Value<'static>>,
}

impl<'a> ResultRowRef<'a> {
    pub fn iter(&self) -> impl Iterator<Item = &'a Value<'a>> {
        self.values.iter()
    }
}

impl ResultRow {
    /// Take a value from a certain position in the row, if having a value in
    /// that position. Usage documentation in
    /// [ResultRowRef](struct.ResultRowRef.html).
    pub fn at(&self, i: usize) -> Option<&Value<'static>> {
        if self.values.len() <= i {
            None
        } else {
            Some(&self.values[i])
        }
    }

    /// Take a value with the given column name from the row. Usage
    /// documentation in [ResultRowRef](struct.ResultRowRef.html).
    pub fn get(&self, name: &str) -> Option<&Value<'static>> {
        self.columns.iter().position(|c| c == name).map(|idx| &self.values[idx])
    }

    /// Make a referring [ResultRowRef](struct.ResultRowRef.html).
    pub fn as_ref(&self) -> ResultRowRef {
        ResultRowRef {
            columns: Arc::clone(&self.columns),
            values: &self.values,
            types: self.types.clone(),
        }
    }

    pub fn into_single(self) -> crate::Result<Value<'static>> {
        match self.into_iter().next() {
            Some(val) => Ok(val),
            None => Err(Error::builder(ErrorKind::NotFound).build()),
        }
    }
}

impl<'a> ResultRowRef<'a> {
    /// Take a value from a certain position in the row, if having a value in
    /// that position.
    ///
    /// ```
    /// # use quaint::connector::*;
    /// # let names = vec!["id".to_string(), "name".to_string()];
    /// # let rows = vec![vec!["1234".into(), "Musti".into()]];
    /// # let result_set = ResultSet::new(names, rows);
    /// # let row = result_set.first().unwrap();
    /// assert_eq!(Some(&row[0]), row.at(0));
    /// ```
    pub fn at(&self, i: usize) -> Option<&'a Value<'static>> {
        if self.values.len() <= i {
            None
        } else {
            Some(&self.values[i])
        }
    }

    /// Take a value with the given column name from the row.
    ///
    /// ```
    /// # use quaint::connector::*;
    /// # let names = vec!["id".to_string(), "name".to_string()];
    /// # let rows = vec![vec!["1234".into(), "Musti".into()]];
    /// # let result_set = ResultSet::new(names, rows);
    /// # let row = result_set.first().unwrap();
    /// assert_eq!(Some(&row["id"]), row.get("id"));
    /// ```
    pub fn get(&self, name: &str) -> Option<&'a Value<'static>> {
        self.columns.iter().position(|c| c == name).map(|idx| &self.values[idx])
    }

    /// Returns the length of the row.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns whether the rows are empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = &'a Value<'static>> {
        self.values.iter()
    }
}
