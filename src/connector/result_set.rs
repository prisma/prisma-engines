mod index;
mod result_row;

pub use index::*;
pub use result_row::*;

use crate::ast::ParameterizedValue;
use std::{
    collections::{btree_map::Keys, BTreeMap},
    sync::Arc,
};

#[cfg(feature = "json-1")]
use serde_json::{Map, Value};

/// Encapsulates a set of results and their respective column names.
#[derive(Debug, Default)]
pub struct ResultSet {
    pub(crate) rows: Vec<Vec<ParameterizedValue<'static>>>,
    pub(crate) name_to_index: Arc<BTreeMap<String, usize>>,
    pub(crate) last_insert_id: Option<u64>,
}

impl ResultSet {
    /// Creates a new instance, bound to the given column names and result rows.
    pub fn new(
        names: Vec<String>,
        rows: Vec<Vec<ParameterizedValue<'static>>>,
    ) -> Self
    {
        Self {
            name_to_index: Arc::new(Self::build_name_map(names)),
            rows,
            last_insert_id: None,
        }
    }

    pub(crate) fn set_last_insert_id(&mut self, id: u64) {
        self.last_insert_id = Some(id);
    }

    /// The last id inserted, if available. Only works on certain databases and
    /// if using an auto-increment ids.
    pub fn last_insert_id(&self) -> Option<u64> {
        self.last_insert_id
    }

    /// An iterator of column names.
    pub fn columns(&self) -> Keys<'_, String, usize> {
        self.name_to_index.keys()
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
            name_to_index: Arc::clone(&self.name_to_index),
            values: row,
        })
    }

    /// Creates a lookup map for column names.
    fn build_name_map(names: Vec<String>) -> BTreeMap<String, usize> {
        names
            .into_iter()
            .enumerate()
            .fold(BTreeMap::new(), |mut acc, (i, name)| {
                acc.insert(name, i);
                acc
            })
    }
}

impl IntoIterator for ResultSet {
    type Item = ResultRow;
    type IntoIter = ResultSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        ResultSetIterator {
            name_to_index: self.name_to_index,
            internal_iterator: self.rows.into_iter(),
        }
    }
}

/// Thin iterator for ResultSet rows.
/// Might become lazy one day.
pub struct ResultSetIterator {
    pub(crate) name_to_index: Arc<BTreeMap<String, usize>>,
    pub(crate) internal_iterator: std::vec::IntoIter<Vec<ParameterizedValue<'static>>>,
}

impl Iterator for ResultSetIterator {
    type Item = ResultRow;

    fn next(&mut self) -> Option<Self::Item> {
        match self.internal_iterator.next() {
            Some(row) => Some(ResultRow {
                name_to_index: Arc::clone(&self.name_to_index),
                values: row,
            }),
            None => None,
        }
    }
}

#[cfg(feature = "json-1")]
impl From<ResultSet> for Value {
    fn from(result_set: ResultSet) -> Self {
        let columns: Vec<String> = result_set.columns().map(ToString::to_string).collect();
        let mut result = Vec::new();

        for row in result_set.into_iter() {
            let mut object = Map::new();

            for (idx, p_value) in row.into_iter().enumerate() {
                let column_name: String = columns[idx].clone();
                object.insert(column_name, Value::from(p_value));
            }

            result.push(Value::Object(object));
        }

        Value::Array(result)
    }
}
