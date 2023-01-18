use quaint::prelude::ConnectionInfo;
use std::env;

#[cfg(not(test))]
fn get_batch_size(default: usize) -> Option<usize> {
    use once_cell::sync::Lazy;

    /// Overrides the default number of allowed elements in query's `IN` or `NOT IN`
    /// statement for the currently loaded connector.
    /// Certain databases error out if querying with too many items. For test
    /// purposes, this value can be set with the `QUERY_BATCH_SIZE` environment
    /// value to a smaller number.
    static BATCH_SIZE_OVERRIDE: Lazy<Option<usize>> = Lazy::new(|| {
        env::var("QUERY_BATCH_SIZE")
            .ok()
            .map(|size| size.parse().expect("QUERY_BATCH_SIZE: not a valid size"))
    });
    (*BATCH_SIZE_OVERRIDE).or(Some(default))
}

#[cfg(test)]
fn get_batch_size(_: usize) -> Option<usize> {
    env::var("QUERY_BATCH_SIZE").ok().and_then(|size| size.parse().ok())
}

/// Contains meta information about the loaded connector.
pub(crate) struct SqlInfo {
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
            max_rows: Some(999),
            max_bind_values: get_batch_size(999),
        }
    }

    fn mysql() -> Self {
        Self {
            max_rows: None,
            // See https://stackoverflow.com/a/11131824/788562
            max_bind_values: get_batch_size(65535),
        }
    }

    fn postgres() -> Self {
        Self {
            max_rows: None,
            max_bind_values: get_batch_size(32766),
        }
    }

    fn mssql() -> Self {
        Self {
            max_rows: Some(1000),
            max_bind_values: get_batch_size(2099),
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
