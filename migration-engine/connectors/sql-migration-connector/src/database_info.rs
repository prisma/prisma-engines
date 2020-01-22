use super::SqlResult;
use quaint::{
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};

#[derive(Debug, Clone)]
pub struct DatabaseInfo {
    connection_info: ConnectionInfo,
    database_version: Option<String>,
}

impl DatabaseInfo {
    pub(crate) async fn new(connection: &Quaint, connection_info: ConnectionInfo) -> SqlResult<Self> {
        let database_version = get_database_version(connection, &connection_info).await?;

        Ok(DatabaseInfo {
            connection_info,
            database_version,
        })
    }

    pub(crate) fn is_mariadb(&self) -> bool {
        self.connection_info.sql_family() == SqlFamily::Mysql
            && self
                .database_version
                .as_ref()
                .map(|version| version.contains("MariaDB"))
                .unwrap_or(false)
    }

    pub(crate) fn sql_family(&self) -> SqlFamily {
        self.connection_info.sql_family()
    }

    pub(crate) fn connection_info(&self) -> &ConnectionInfo {
        &self.connection_info
    }
}

async fn get_database_version(connection: &Quaint, connection_info: &ConnectionInfo) -> SqlResult<Option<String>> {
    match connection_info.sql_family() {
        SqlFamily::Mysql => {
            let query = r#"SELECT @@GLOBAL.version version"#;

            let rows = connection.query_raw(query, &[]).await?;

            let version_string = rows
                .get(0)
                .and_then(|row| row.get("version").and_then(|version| version.to_string()));

            Ok(version_string)
        }
        _ => Ok(None),
    }
}
