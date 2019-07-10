mod index;
mod result_row;

pub use index::*;
pub use result_row::*;

use crate::ast::ParameterizedValue;
use std::{collections::HashMap, sync::Arc};

/// Encapsulates a set of results and their respective column names.
#[derive(Debug)]
pub struct ResultSet {
    pub(crate) rows: Vec<Vec<ParameterizedValue<'static>>>,
    pub(crate) name_to_index: Arc<HashMap<String, usize>>,
}

impl ResultSet {
    /// Creates a new instance, bound to the given column names and result rows.
    pub fn new(names: Vec<String>, rows: Vec<Vec<ParameterizedValue<'static>>>) -> ResultSet {
        ResultSet {
            name_to_index: Arc::new(Self::build_name_map(names)),
            rows,
        }
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
    pub fn first<'a>(&'a self) -> Option<ResultRowRef<'a>> {
        self.get(0)
    }

    /// Returns a reference to a row in a given position.
    pub fn get<'a>(&'a self, index: usize) -> Option<ResultRowRef<'a>> {
        self.rows.get(index).map(|row| ResultRowRef {
            name_to_index: Arc::clone(&self.name_to_index),
            values: row,
        })
    }

    /// Creates a lookup map for column names.
    fn build_name_map(names: Vec<String>) -> HashMap<String, usize> {
        names
            .into_iter()
            .enumerate()
            .fold(HashMap::new(), |mut acc, (i, name)| {
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
    pub(crate) name_to_index: Arc<HashMap<String, usize>>,
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
