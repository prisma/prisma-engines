use crate::ast::{Column, Ordering};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Over<'a> {
    pub(crate) ordering: Ordering<'a>,
    pub(crate) partitioning: Vec<Column<'a>>,
}

impl<'a> Over<'a> {
    pub fn is_empty(&self) -> bool {
        self.ordering.is_empty() && self.partitioning.is_empty()
    }
}
