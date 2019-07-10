use crate::{ast::ParameterizedValue, connector::transaction::Row};
use std::{collections::HashMap, sync::Arc};

/// An owned version of a `Row` in a `ResultSet`. See
/// [ResultRowRef](struct.ResultRowRef.html) for documentation on data access.
pub struct ResultRow {
    pub(crate) name_to_index: Arc<HashMap<String, usize>>,
    pub(crate) values: Row,
}

/// A reference to a `Row` in a `ResultSet`. The columns can be accessed either
/// through their position or using the column name.
///
/// ```
/// # use prisma_query::connector::*;
/// let names = ColumnNames::from(vec!["id", "name"]);
/// let rows = vec![Row::from(vec!["1234", "Musti"])];
///
/// let result_set = ResultSet::new(names, rows);
/// let row = result_set.first().unwrap();
///
/// assert_eq!(row[0], row["id"]);
/// assert_eq!(row[1], row["name"]);
/// ```
pub struct ResultRowRef<'a> {
    pub(crate) name_to_index: Arc<HashMap<String, usize>>,
    pub(crate) values: &'a Row,
}

impl ResultRow {
    pub fn at(&self, i: usize) -> Option<&ParameterizedValue<'static>> {
        if self.values.values.len() <= i {
            None
        } else {
            Some(&self.values.values[i])
        }
    }

    pub fn get(&self, name: &str) -> Option<&ParameterizedValue<'static>> {
        if let Some(idx) = self.name_to_index.get(name) {
            Some(&self.values.values[*idx])
        } else {
            None
        }
    }

    pub fn as_ref<'a>(&'a self) -> ResultRowRef<'a> {
        ResultRowRef {
            name_to_index: Arc::clone(&self.name_to_index),
            values: &self.values,
        }
    }
}

impl<'a> ResultRowRef<'a> {
    pub fn at(&self, i: usize) -> Option<&ParameterizedValue<'static>> {
        if self.values.values.len() <= i {
            None
        } else {
            Some(&self.values.values[i])
        }
    }

    pub fn get(&self, name: &str) -> Option<&ParameterizedValue<'static>> {
        if let Some(idx) = self.name_to_index.get(name) {
            Some(&self.values.values[*idx])
        } else {
            None
        }
    }
}
