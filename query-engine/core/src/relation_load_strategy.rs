use connector::error::{ConnectorError, ErrorKind};

use crate::CoreError;

/// Returns whether the database supports joins given its version.
/// Only versions of the MySQL connector are currently parsed at runtime.
pub fn db_version_supports_joins_strategy(db_version: Option<String>) -> crate::Result<bool> {
    DatabaseVersion::try_from(db_version.as_deref()).map(|version| version.supports_join_relation_load_strategy())
}

/// Parsed database version.
#[derive(Debug)]
enum DatabaseVersion {
    Mysql(u16, u16, u16),
    Mariadb,
    Unknown,
}

impl DatabaseVersion {
    /// Returns whether the database supports joins given its version.
    /// Only versions of the MySQL connector are currently parsed at runtime.
    pub(crate) fn supports_join_relation_load_strategy(&self) -> bool {
        match self {
            // Prior to MySQL 8.0.14, a derived table cannot contain outer references.
            // Source: https://dev.mysql.com/doc/refman/8.0/en/derived-tables.html
            DatabaseVersion::Mysql(major, minor, patch) => (*major, *minor, *patch) >= (8, 0, 14),
            DatabaseVersion::Mariadb => false,
            DatabaseVersion::Unknown => true,
        }
    }
}

impl TryFrom<Option<&str>> for DatabaseVersion {
    type Error = crate::CoreError;

    fn try_from(version: Option<&str>) -> crate::Result<Self> {
        match version {
            Some(version) => {
                let build_err = |reason: &str| {
                    CoreError::ConnectorError(ConnectorError::from_kind(ErrorKind::UnexpectedDatabaseVersion {
                        version: version.into(),
                        reason: reason.into(),
                    }))
                };

                let mut iter = version.split('-');

                let version = iter.next().ok_or_else(|| build_err("Missing version"))?;
                let is_mariadb = iter.next().map(|s| s.contains("MariaDB")).unwrap_or(false);

                if is_mariadb {
                    return Ok(DatabaseVersion::Mariadb);
                }

                let mut version_iter = version.split('.');

                let major = version_iter.next().ok_or_else(|| build_err("Missing major version"))?;
                let minor = version_iter.next().ok_or_else(|| build_err("Missing minor version"))?;
                let patch = version_iter.next().ok_or_else(|| build_err("Missing patch version"))?;

                let parsed_major = major.parse().map_err(|_| build_err("Major version is not a number"))?;
                let parsed_minor = minor.parse().map_err(|_| build_err("Minor version is not a number"))?;
                let parsed_patch = patch.parse().map_err(|_| build_err("Patch version is not a number"))?;

                Ok(DatabaseVersion::Mysql(parsed_major, parsed_minor, parsed_patch))
            }
            None => Ok(DatabaseVersion::Unknown),
        }
    }
}
