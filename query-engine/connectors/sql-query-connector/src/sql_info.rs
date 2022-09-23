use once_cell::sync::Lazy;
use psl::datamodel_connector::{ConnectorCapabilities, ConnectorCapability};
use quaint::prelude::ConnectionInfo;
use std::env;

/// Overrides the default number of allowed elements in query's `IN` or `NOT IN`
/// statement for the currently loaded connector.
/// Certain databases error out if querying with too many items. For test
/// purposes, this value can be set with the `QUERY_BATCH_SIZE` environment
/// value to a smaller number.
pub static BATCH_SIZE_OVERRIDE: Lazy<Option<usize>> =
    Lazy::new(|| env::var("QUERY_BATCH_SIZE").ok().and_then(|size| size.parse().ok()));

#[cfg(not(test))]
fn get_batch_size(default: usize) -> Option<usize> {
    (*BATCH_SIZE_OVERRIDE).or(Some(default))
}

#[cfg(test)]
fn get_batch_size(_: usize) -> Option<usize> {
    env::var("QUERY_BATCH_SIZE").ok().and_then(|size| size.parse().ok())
}

pub enum SqlFamily {
    SQLite,
    Postgres,
    MySQL,
    MSSQL,
}

/// Contains meta information about the loaded connector.
pub struct SqlInfo {
    /// SQL family the connector belongs to.
    pub family: SqlFamily,

    /// Maximum rows allowed at once for an insert query.
    /// None is unlimited.
    pub max_rows: Option<usize>,

    /// Maximum number of bind parameters allowed for a single query.
    /// None is unlimited.
    pub max_bind_values: Option<usize>,

    /// Capabilities of the connector
    pub capabilities: ConnectorCapabilities,
}

impl SqlInfo {
    #[allow(dead_code)]
    pub fn has_capability(&self, capability: ConnectorCapability) -> bool {
        self.capabilities.contains(capability)
    }

    fn sqlite() -> Self {
        Self {
            family: SqlFamily::SQLite,
            max_rows: Some(999),
            max_bind_values: get_batch_size(999),
            capabilities: ConnectorCapabilities::new(psl::builtin_connectors::SQLITE.capabilities().to_owned()),
        }
    }

    fn mysql() -> Self {
        Self {
            family: SqlFamily::MySQL,
            max_rows: None,
            // See https://stackoverflow.com/a/11131824/788562
            max_bind_values: get_batch_size(65535),
            capabilities: ConnectorCapabilities::new(psl::builtin_connectors::MYSQL.capabilities().to_owned()),
        }
    }

    fn postgres() -> Self {
        Self {
            family: SqlFamily::Postgres,
            max_rows: None,
            max_bind_values: get_batch_size(32766),
            capabilities: ConnectorCapabilities::new(psl::builtin_connectors::POSTGRES.capabilities().to_owned()),
        }
    }

    fn mssql() -> Self {
        Self {
            family: SqlFamily::MSSQL,
            max_rows: Some(1000),
            max_bind_values: get_batch_size(2099),
            capabilities: ConnectorCapabilities::new(psl::builtin_connectors::MSSQL.capabilities().to_owned()),
        }
    }
}

impl From<&ConnectionInfo> for SqlInfo {
    fn from(ci: &ConnectionInfo) -> Self {
        match ci {
            ConnectionInfo::Postgres(_) => Self::postgres(),
            ConnectionInfo::Mysql(_) => Self::mysql(),
            ConnectionInfo::Mssql(_) => Self::mssql(),
            ConnectionInfo::Sqlite { .. } => Self::sqlite(),
            ConnectionInfo::InMemorySqlite { .. } => Self::sqlite(),
        }
    }
}
