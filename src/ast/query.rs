use crate::ast::{Insert, Select};

/// A database query
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    /// Query for fetching data. E.g. the `SELECT` query.
    Select(Select),
    Insert(Insert),
}
