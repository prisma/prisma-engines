mod connector_transaction;
mod mysql;
mod postgresql;
mod sqlite;

use crate::query_builder::*;
use datamodel::Source;
use connector_interface::Connector;

pub use mysql::*;
pub use postgresql::*;
pub use sqlite::*;

pub trait FromSource {
    fn from_source(source: &dyn Source) -> crate::Result<Self>
    where
        Self: Connector + SqlCapabilities + Sized;
}

pub trait SqlCapabilities {
    /// This we use to differentiate between databases with or without
    /// `ROW_NUMBER` function for related records pagination.
    type ManyRelatedRecordsBuilder: ManyRelatedRecordsQueryBuilder;
}

/// A wrapper for relational databases due to trait restrictions. Implements the
/// needed traits.
pub struct SqlDatabase<T>
where
    T: Connector + SqlCapabilities,
{
    pub executor: T,
}

impl<T> SqlDatabase<T>
where
    T: Connector + SqlCapabilities,
{
    pub fn new(executor: T) -> Self {
        Self { executor }
    }
}
