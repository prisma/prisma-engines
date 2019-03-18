use crate::ast::{Delete, Insert, Select, Update};

/// A database query
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    /// Query for fetching data. E.g. the `SELECT` query.
    Select(Select),
    Insert(Insert),
    Update(Update),
    Delete(Delete),
}
