use crate::ast::ParameterizedValue;
use std::sync::Arc;

/// An owned version of a `Row` in a `ResultSet`. See
/// [ResultRowRef](struct.ResultRowRef.html) for documentation on data access.
#[derive(Debug)]
pub struct ResultRow {
    pub(crate) columns: Arc<Vec<String>>,
    pub(crate) values: Vec<ParameterizedValue<'static>>,
}

impl IntoIterator for ResultRow {
    type Item = ParameterizedValue<'static>;
    type IntoIter = std::vec::IntoIter<ParameterizedValue<'static>>;

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
#[derive(Debug)]
pub struct ResultRowRef<'a> {
    pub(crate) columns: Arc<Vec<String>>,
    pub(crate) values: &'a Vec<ParameterizedValue<'static>>,
}

impl ResultRow {
    /// Take a value from a certain position in the row, if having a value in
    /// that position. Usage documentation in
    /// [ResultRowRef](struct.ResultRowRef.html).
    pub fn at(&self, i: usize) -> Option<&ParameterizedValue<'static>> {
        if self.values.len() <= i {
            None
        } else {
            Some(&self.values[i])
        }
    }

    /// Take a value with the given column name from the row. Usage
    /// documentation in [ResultRowRef](struct.ResultRowRef.html).
    pub fn get(&self, name: &str) -> Option<&ParameterizedValue<'static>> {
        if let Some(idx) = self.columns.iter().position(|c| c == name) {
            Some(&self.values[idx])
        } else {
            None
        }
    }

    /// Make a referring [ResultRowRef](struct.ResultRowRef.html).
    pub fn as_ref(&self) -> ResultRowRef {
        ResultRowRef {
            columns: Arc::clone(&self.columns),
            values: &self.values,
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
    pub fn at(&self, i: usize) -> Option<&ParameterizedValue<'static>> {
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
    pub fn get(&self, name: &str) -> Option<&ParameterizedValue<'static>> {
        if let Some(idx) = self.columns.iter().position(|c| c == name) {
            Some(&self.values[idx])
        } else {
            None
        }
    }
}
