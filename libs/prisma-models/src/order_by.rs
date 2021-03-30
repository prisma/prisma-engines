use crate::{RelationFieldRef, ScalarFieldRef};
use quaint::prelude::Order;
use std::string::ToString;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderBy {
    pub field: ScalarFieldRef,
    pub path: Vec<RelationFieldRef>,
    pub sort_order: SortOrder,
    pub sort_aggregation: Option<SortAggregation>,
}

impl OrderBy {
    pub fn new(
        field: ScalarFieldRef,
        path: Vec<RelationFieldRef>,
        sort_order: SortOrder,
        sort_aggregation: Option<SortAggregation>,
    ) -> Self {
        Self {
            field,
            path,
            sort_order,
            sort_aggregation,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    pub fn into_order(&self, reverse: bool) -> Order {
        match (self, reverse) {
            (SortOrder::Ascending, false) => Order::Asc,
            (SortOrder::Descending, false) => Order::Desc,
            (SortOrder::Ascending, true) => Order::Desc,
            (SortOrder::Descending, true) => Order::Asc,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum SortAggregation {
    Count,
    Avg,
    Sum,
    Min,
    Max,
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
            path: vec![],
            sort_order: SortOrder::Ascending,
            sort_aggregation: None,
        }
    }
}
