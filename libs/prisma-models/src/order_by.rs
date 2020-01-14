use crate::{ModelRef, ScalarField};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OrderBy {
    pub field: Arc<ScalarField>,
    pub sort_order: SortOrder,
}

pub trait IntoOrderBy {
    fn into_order_by(self, model: ModelRef) -> OrderBy;
}

#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    /// "ASC" / "DESC"
    pub fn abbreviated(self) -> &'static str {
        match self {
            SortOrder::Ascending => "ASC",
            SortOrder::Descending => "DESC",
        }
    }
}
