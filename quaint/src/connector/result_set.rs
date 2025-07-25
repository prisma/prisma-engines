mod index;
mod result_row;

pub use index::*;
pub use result_row::*;

use crate::{ast::Value, error::*};
use serde_json::Map;
use std::sync::Arc;

use super::ColumnType;

/// Encapsulates a set of results and their respective column names.
#[derive(Debug, Default)]
pub struct ResultSet {
    pub(crate) columns: Arc<Vec<String>>,
    pub(crate) types: Vec<ColumnType>,
    pub(crate) rows: Vec<Vec<Value<'static>>>,
    pub(crate) last_insert_id: Option<u64>,
}

impl ResultSet {
    /// Creates a new instance, bound to the given column names and result rows.
    pub fn new(names: Vec<String>, types: Vec<ColumnType>, rows: Vec<Vec<Value<'static>>>) -> Self {
        Self {
            columns: Arc::new(names),
            types,
            rows,
            last_insert_id: None,
        }
    }

    pub fn set_last_insert_id(&mut self, id: u64) {
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
    pub fn first(&self) -> Option<ResultRowRef<'_>> {
        self.get(0)
    }

    /// Returns a reference to a row in a given position.
    pub fn get(&self, index: usize) -> Option<ResultRowRef<'_>> {
        self.rows.get(index).map(|row| ResultRowRef {
            columns: Arc::clone(&self.columns),
            values: row,
            types: self.types.clone(),
        })
    }

    /// Takes the first row if existing, otherwise returns error.
    pub fn into_single(self) -> crate::Result<ResultRow> {
        match self.into_iter().next() {
            Some(row) => Ok(row),
            None => Err(Error::builder(ErrorKind::NotFound).build()),
        }
    }

    pub fn iter(&self) -> ResultSetIterator<'_> {
        ResultSetIterator {
            columns: self.columns.clone(),
            types: self.types.clone(),
            internal_iterator: self.rows.iter(),
        }
    }

    pub fn types(&self) -> &[ColumnType] {
        &self.types
    }
}

impl IntoIterator for ResultSet {
    type Item = ResultRow;
    type IntoIter = ResultSetIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        ResultSetIntoIterator {
            columns: self.columns,
            types: self.types.clone(),
            internal_iterator: self.rows.into_iter(),
        }
    }
}

/// Thin iterator for ResultSet rows.
/// Might become lazy one day.
pub struct ResultSetIntoIterator {
    pub(crate) columns: Arc<Vec<String>>,
    pub(crate) types: Vec<ColumnType>,
    pub(crate) internal_iterator: std::vec::IntoIter<Vec<Value<'static>>>,
}

impl Iterator for ResultSetIntoIterator {
    type Item = ResultRow;

    fn next(&mut self) -> Option<Self::Item> {
        match self.internal_iterator.next() {
            Some(row) => Some(ResultRow {
                columns: Arc::clone(&self.columns),
                values: row,
                types: self.types.clone(),
            }),
            None => None,
        }
    }
}

pub struct ResultSetIterator<'a> {
    pub(crate) columns: Arc<Vec<String>>,
    pub(crate) types: Vec<ColumnType>,
    pub(crate) internal_iterator: std::slice::Iter<'a, Vec<Value<'static>>>,
}

impl<'a> Iterator for ResultSetIterator<'a> {
    type Item = ResultRowRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.internal_iterator.next() {
            Some(row) => Some(ResultRowRef {
                columns: Arc::clone(&self.columns),
                values: row,
                types: self.types.clone(),
            }),
            None => None,
        }
    }
}

impl From<ResultSet> for serde_json::Value {
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
