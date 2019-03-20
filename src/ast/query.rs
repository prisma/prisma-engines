use crate::ast::{Delete, Insert, Select, Update};

/// A database query
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    /// Query for fetching data. E.g. the `SELECT` query.
    Select(Select),
    Insert(Box<Insert>),
    Update(Box<Update>),
    Delete(Box<Delete>),
}

impl Query {
    pub fn is_select(&self) -> bool {
        if let Query::Select(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_insert(&self) -> bool {
        if let Query::Insert(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_update(&self) -> bool {
        if let Query::Update(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_delete(&self) -> bool {
        if let Query::Delete(_) = self {
            true
        } else {
            false
        }
    }
}
