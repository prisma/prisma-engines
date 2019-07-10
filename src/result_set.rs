use crate::{
    ast::ParameterizedValue,
    transaction::{ColumnNames, Row},
};
use std::{collections::HashMap, ops, sync::Arc};

/// Encapsulates a set of results and their respective column names.
#[derive(Debug)]
pub struct ResultSet {
    pub(crate) rows: Vec<Row>,
    pub(crate) name_to_index: Arc<HashMap<String, usize>>,
}

impl ResultSet {
    /// Creates a new instance, bound to the given column names and result rows.
    pub fn new(names: ColumnNames, rows: Vec<Row>) -> ResultSet {
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
    fn build_name_map(names: ColumnNames) -> HashMap<String, usize> {
        names
            .names
            .into_iter()
            .enumerate()
            .fold(HashMap::new(), |mut acc, (i, name)| {
                acc.insert(name, i);
                acc
            })
    }
}

/// A reference to a `Row` in a `ResultSet`. The columns can be accessed either
/// through their position in the `Row` or using the column name.
///
/// ```
/// # use prisma_query::*;
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
    name_to_index: Arc<HashMap<String, usize>>,
    values: &'a Row,
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
    name_to_index: Arc<HashMap<String, usize>>,
    internal_iterator: std::vec::IntoIter<Row>,
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

pub struct ResultRow {
    name_to_index: Arc<HashMap<String, usize>>,
    values: Row,
}

pub trait ValueIndex<RowType>: private::Sealed {
    #[doc(hidden)]
    fn index_into<'a>(self, row: &'a RowType) -> &'a ParameterizedValue<'static>;
}

// Prevent users from implementing the ValueIndex trait.
mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for &str {}
}

impl ValueIndex<ResultRowRef<'_>> for usize {
    fn index_into<'v>(self, row: &'v ResultRowRef) -> &'v ParameterizedValue<'static> {
        row.at(self).unwrap()
    }
}

impl ValueIndex<ResultRowRef<'_>> for &str {
    fn index_into<'v>(self, row: &'v ResultRowRef) -> &'v ParameterizedValue<'static> {
        row.get(self).unwrap()
    }
}

impl ValueIndex<ResultRow> for usize {
    fn index_into<'v>(self, row: &'v ResultRow) -> &'v ParameterizedValue<'static> {
        row.at(self).unwrap()
    }
}

impl ValueIndex<ResultRow> for &str {
    fn index_into<'v>(self, row: &'v ResultRow) -> &'v ParameterizedValue<'static> {
        row.get(self).unwrap()
    }
}

impl<'a, I: ValueIndex<ResultRowRef<'a>> + 'static> ops::Index<I> for ResultRowRef<'a> {
    type Output = ParameterizedValue<'static>;

    fn index(&self, index: I) -> &ParameterizedValue<'static> {
        index.index_into(self)
    }
}

impl<I: ValueIndex<ResultRow> + 'static> ops::Index<I> for ResultRow {
    type Output = ParameterizedValue<'static>;

    fn index(&self, index: I) -> &ParameterizedValue<'static> {
        index.index_into(self)
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
