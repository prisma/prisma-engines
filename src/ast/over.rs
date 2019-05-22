use crate::ast::{Column, Ordering};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Over {
    pub(crate) ordering: Ordering,
    pub(crate) partitioning: Vec<Column>,
}

impl Over {
    pub fn is_empty(&self) -> bool {
        self.ordering.is_empty() && self.partitioning.is_empty()
    }
}