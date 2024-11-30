use crate::ast::{Column, Ordering};

#[derive(Debug, Default, Clone, PartialEq)]
/// Determines the partitioning and ordering of a rowset before the associated
/// window function is applied.
pub struct Over<'a> {
    pub(crate) ordering: Ordering<'a>,
    pub(crate) partitioning: Vec<Column<'a>>,
}

impl Over<'_> {
    pub fn is_empty(&self) -> bool {
        self.ordering.is_empty() && self.partitioning.is_empty()
    }
}
