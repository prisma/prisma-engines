use once_cell::sync::Lazy;
use quaint::prelude::ConnectionInfo;
use std::env;

/// Overrides the default number of allowed elements in query's `IN` or `NOT IN`
/// statement for the currently loaded connector.
/// Certain databases error out if querying with too many items. For test
/// purposes, this value can be set with the `QUERY_BATCH_SIZE` environment
/// value to a smaller number.
pub static BATCH_SIZE_OVERRIDE: Lazy<Option<usize>> =
    Lazy::new(|| env::var("QUERY_BATCH_SIZE").ok().and_then(|size| size.parse().ok()));

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
}

impl SqlInfo {
    fn sqlite() -> Self {
        Self {
            family: SqlFamily::SQLite,
            max_rows: Some(999),
            max_bind_values: (*BATCH_SIZE_OVERRIDE).or(Some(999)),
        }
    }

    fn mysql() -> Self {
        Self {
            family: SqlFamily::MySQL,
            max_rows: None,
            // See https://stackoverflow.com/a/11131824/788562
            max_bind_values: (*BATCH_SIZE_OVERRIDE).or(Some(65535)),
        }
    }

    fn postgres() -> Self {
        Self {
            family: SqlFamily::Postgres,
            max_rows: None,
            max_bind_values: (*BATCH_SIZE_OVERRIDE).or(Some(32767)),
        }
    }

    fn mssql() -> Self {
        Self {
            family: SqlFamily::MSSQL,
            max_rows: Some(1000),
            max_bind_values: (*BATCH_SIZE_OVERRIDE).or(Some(2099)),
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
