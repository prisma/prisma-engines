use crate::ast::ParameterizedValue;
use std::{collections::HashMap, sync::Arc};

/// An owned version of a `Row` in a `ResultSet`. See
/// [ResultRowRef](struct.ResultRowRef.html) for documentation on data access.
pub struct ResultRow {
    pub(crate) name_to_index: Arc<HashMap<String, usize>>,
    pub(crate) values: Vec<ParameterizedValue<'static>>,
}

/// A reference to a `Row` in a `ResultSet`. The columns can be accessed either
/// through their position or using the column name.
///
/// ```
/// # use prisma_query::connector::*;
/// let names = vec!["id".to_string(), "name".to_string()];
/// let rows = vec![vec!["1234".into(), "Musti".into()]];
///
/// let result_set = ResultSet::new(names, rows);
/// let row = result_set.first().unwrap();
///
/// assert_eq!(row[0], row["id"]);
/// assert_eq!(row[1], row["name"]);
/// ```
pub struct ResultRowRef<'a> {
    pub(crate) name_to_index: Arc<HashMap<String, usize>>,
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
        if let Some(idx) = self.name_to_index.get(name) {
            Some(&self.values[*idx])
        } else {
            None
        }
    }

    /// Make a referring [ResultRowRef](struct.ResultRowRef.html).
    pub fn as_ref<'a>(&'a self) -> ResultRowRef<'a> {
        ResultRowRef {
            name_to_index: Arc::clone(&self.name_to_index),
            values: &self.values,
        }
    }
}

impl<'a> ResultRowRef<'a> {
    /// Take a value from a certain position in the row, if having a value in
    /// that position.
    ///
    /// ```
    /// # use prisma_query::connector::*;
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
    /// # use prisma_query::connector::*;
    /// # let names = vec!["id".to_string(), "name".to_string()];
    /// # let rows = vec![vec!["1234".into(), "Musti".into()]];
    /// # let result_set = ResultSet::new(names, rows);
    /// # let row = result_set.first().unwrap();
    /// assert_eq!(Some(&row["id"]), row.get("id"));
    /// ```
    pub fn get(&self, name: &str) -> Option<&ParameterizedValue<'static>> {
        if let Some(idx) = self.name_to_index.get(name) {
            Some(&self.values[*idx])
        } else {
            None
        }
    }
}
