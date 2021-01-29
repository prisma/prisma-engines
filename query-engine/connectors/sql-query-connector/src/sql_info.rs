use quaint::prelude::ConnectionInfo;

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
            max_bind_values: Some(999),
        }
    }

    fn mysql() -> Self {
        Self {
            family: SqlFamily::MySQL,
            max_rows: None,
            max_bind_values: None,
        }
    }

    fn postgres() -> Self {
        Self {
            family: SqlFamily::Postgres,
            max_rows: None,
            max_bind_values: Some(32767),
        }
    }

    fn mssql() -> Self {
        Self {
            family: SqlFamily::MSSQL,
            max_rows: Some(1000),
            max_bind_values: Some(2099),
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
