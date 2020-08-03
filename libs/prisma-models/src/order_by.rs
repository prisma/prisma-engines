use crate::{ModelRef, ScalarFieldRef};
use std::string::ToString;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderBy {
    pub field: ScalarFieldRef,
    pub sort_order: SortOrder,
}

impl OrderBy {
    pub fn new(field: ScalarFieldRef, sort_order: SortOrder) -> Self {
        Self { field, sort_order }
    }
}

pub trait IntoOrderBy {
    fn into_order_by(self, model: ModelRef) -> OrderBy;
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl ToString for SortOrder {
    fn to_string(&self) -> String {
        match self {
            SortOrder::Ascending => String::from("ASC"),
            SortOrder::Descending => String::from("DESC"),
        }
    }
}

impl From<ScalarFieldRef> for OrderBy {
    fn from(field: ScalarFieldRef) -> Self {
        Self {
            field,
            sort_order: SortOrder::Ascending,
        }
    }
}
