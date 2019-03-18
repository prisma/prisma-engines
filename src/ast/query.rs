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
