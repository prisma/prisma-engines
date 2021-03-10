mod index;
mod result_row;

pub use index::*;
pub use result_row::*;

use crate::{ast::Value, error::*};
use std::sync::Arc;

#[cfg(feature = "json")]
use serde_json::Map;

/// Encapsulates a set of results and their respective column names.
#[derive(Debug, Default)]
pub struct ResultSet {
    pub(crate) columns: Arc<Vec<String>>,
    pub(crate) rows: Vec<Vec<Value<'static>>>,
    pub(crate) last_insert_id: Option<u64>,
}

impl ResultSet {
    /// Creates a new instance, bound to the given column names and result rows.
    pub fn new(names: Vec<String>, rows: Vec<Vec<Value<'static>>>) -> Self {
        Self {
            columns: Arc::new(names),
            rows,
            last_insert_id: None,
        }
    }

    #[cfg(any(feature = "sqlite", feature = "mysql"))]
    pub(crate) fn set_last_insert_id(&mut self, id: u64) {
        self.last_insert_id = Some(id);
    }

    /// The last id inserted, if available. Only works on certain databases and
    /// if using an auto-increment ids.
    pub fn last_insert_id(&self) -> Option<u64> {
        self.last_insert_id
    }

    /// An iterator of column names.
    pub fn columns(&self) -> &Vec<String> {
        &self.columns
    }

    /// Returns the number of rows in the `ResultSet`.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns true if the `ResultSet` contains no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Returns the first row of the `ResultSet`, or None if the set is empty.
    pub fn first(&self) -> Option<ResultRowRef> {
        self.get(0)
    }

    /// Returns a reference to a row in a given position.
    pub fn get(&self, index: usize) -> Option<ResultRowRef> {
        self.rows.get(index).map(|row| ResultRowRef {
            columns: Arc::clone(&self.columns),
            values: row,
        })
    }

    /// Takes the first row if existing, otherwise returns error.
    pub fn into_single(self) -> crate::Result<ResultRow> {
        match self.into_iter().next() {
            Some(row) => Ok(row),
            None => Err(Error::builder(ErrorKind::NotFound).build()),
        }
    }
}

impl IntoIterator for ResultSet {
    type Item = ResultRow;
    type IntoIter = ResultSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        ResultSetIterator {
            columns: self.columns,
            internal_iterator: self.rows.into_iter(),
        }
    }
}

/// Thin iterator for ResultSet rows.
/// Might become lazy one day.
pub struct ResultSetIterator {
    pub(crate) columns: Arc<Vec<String>>,
    pub(crate) internal_iterator: std::vec::IntoIter<Vec<Value<'static>>>,
}

impl Iterator for ResultSetIterator {
    type Item = ResultRow;

    fn next(&mut self) -> Option<Self::Item> {
        match self.internal_iterator.next() {
            Some(row) => Some(ResultRow {
                columns: Arc::clone(&self.columns),
                values: row,
            }),
            None => None,
        }
    }
}

#[cfg(feature = "json")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "json")))]
impl From<ResultSet> for serde_json::Value {
    #[tracing::instrument(name = "result_set_json_conv", skip(result_set))]
    fn from(result_set: ResultSet) -> Self {
        let columns: Vec<String> = result_set.columns().iter().map(ToString::to_string).collect();
        let mut result = Vec::new();

        for row in result_set.into_iter() {
            let mut object = Map::new();

            for (idx, p_value) in row.into_iter().enumerate() {
                let column_name: String = columns[idx].clone();
                object.insert(column_name, serde_json::Value::from(p_value));
            }

            result.push(serde_json::Value::Object(object));
        }

        serde_json::Value::Array(result)
    }
}
