//! SQL flavours implement behaviour specific to a given SQL implementation (PostgreSQL, SQLite...),
//! in order to avoid cluttering the connector with conditionals. This is a private implementation
//! detail of the SQL connector.

use crate::{
    catch, connect, database_info::DatabaseInfo, sql_destructive_change_checker::DestructiveChangeCheckerFlavour,
    sql_renderer::SqlRenderer, sql_schema_differ::SqlSchemaDifferFlavour, CheckDatabaseInfoResult, SqlError, SqlResult,
    SystemDatabase,
};
use futures::future::TryFutureExt;
use migration_connector::{ConnectorError, ConnectorResult};
use once_cell::sync::Lazy;
use quaint::{
    connector::{ConnectionInfo, MysqlUrl, PostgresUrl, Queryable},
    prelude::SqlFamily,
    single::Quaint,
};
use regex::RegexSet;
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use url::Url;

pub(crate) fn from_connection_info(connection_info: &ConnectionInfo) -> Box<dyn SqlFlavour + Send + Sync + 'static> {
    match connection_info {
        ConnectionInfo::Mysql(url) => Box::new(MysqlFlavour(url.clone())),
        ConnectionInfo::Postgres(url) => Box::new(PostgresFlavour(url.clone())),
        ConnectionInfo::Sqlite { file_path, .. } => Box::new(SqliteFlavour {
            file_path: file_path.clone(),
        }),
        ConnectionInfo::Mssql(_) => todo!("Greetings from Redmond!"),
    }
}

#[async_trait::async_trait]
pub(crate) trait SqlFlavour:
    DestructiveChangeCheckerFlavour + SqlRenderer + SqlSchemaDifferFlavour + std::fmt::Debug
{
    /// This method should be considered deprecated. Prefer extending SqlFlavour
    /// with methods expressing clearly what is being specialized by database
    /// backend.
    fn sql_family(&self) -> SqlFamily;

    /// Optionally validate the database info.
    fn check_database_info(&self, _database_info: &DatabaseInfo) -> CheckDatabaseInfoResult {
        Ok(())
    }

    /// Create a database called `dbname` on the server, if applicable.
    async fn create_database(&self, database_url: &str) -> ConnectorResult<String>;

    /// Introspect the SQL schema.
    async fn describe_schema<'a>(
        &'a self,
        schema_name: &'a str,
        conn: Arc<dyn Queryable + Send + Sync>,
    ) -> SqlResult<SqlSchema>;

    /// Create the database schema.
    async fn initialize(&self, conn: &dyn Queryable, database_info: &DatabaseInfo) -> SqlResult<()>;
}

#[derive(Debug)]
pub(crate) struct MysqlFlavour(MysqlUrl);

#[async_trait::async_trait]
impl SqlFlavour for MysqlFlavour {
    fn check_database_info(&self, database_info: &DatabaseInfo) -> CheckDatabaseInfoResult {
        static MYSQL_SYSTEM_DATABASES: Lazy<regex::RegexSet> = Lazy::new(|| {
            RegexSet::new(&[
                "(?i)^mysql$",
                "(?i)^information_schema$",
                "(?i)^performance_schema$",
                "(?i)^sys$",
            ])
            .unwrap()
        });

        let db_name = database_info.connection_info().schema_name();

        if MYSQL_SYSTEM_DATABASES.is_match(db_name) {
            return Err(SystemDatabase(db_name.to_owned()));
        }

        Ok(())
    }

    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        let mut url = Url::parse(database_str).unwrap();
        url.set_path("/mysql");
        let (conn, _) = connect(&url.to_string()).await?;

        let db_name = self.0.dbname();

        let query = format!(
            "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;",
            db_name
        );
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from)).await?;

        Ok(db_name.to_owned())
    }

    async fn describe_schema<'a>(
        &'a self,
        schema_name: &'a str,
        conn: Arc<dyn Queryable + Send + Sync>,
    ) -> SqlResult<SqlSchema> {
        Ok(sql_schema_describer::mysql::SqlSchemaDescriber::new(conn)
            .describe(schema_name)
            .await?)
    }

    async fn initialize(&self, conn: &dyn Queryable, database_info: &DatabaseInfo) -> SqlResult<()> {
        let schema_sql = format!(
            "CREATE SCHEMA IF NOT EXISTS `{}` DEFAULT CHARACTER SET utf8mb4;",
            database_info.connection_info().schema_name()
        );

        conn.raw_cmd(&schema_sql).await?;

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Mysql
    }
}

#[derive(Debug)]
pub(crate) struct SqliteFlavour {
    file_path: String,
}

#[async_trait::async_trait]
impl SqlFlavour for SqliteFlavour {
    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        use anyhow::Context;

        let path = Path::new(&self.file_path);
        if path.exists() {
            return Ok(self.file_path.clone());
        }

        let dir = path.parent();

        if let Some((dir, false)) = dir.map(|dir| (dir, dir.exists())) {
            std::fs::create_dir_all(dir)
                .context("Creating SQLite database parent directory.")
                .map_err(|io_err| ConnectorError::from_kind(migration_connector::ErrorKind::Generic(io_err)))?;
        }

        connect(database_str).await?;

        Ok(self.file_path.clone())
    }

    async fn describe_schema<'a>(
        &'a self,
        schema_name: &'a str,
        conn: Arc<dyn Queryable + Send + Sync>,
    ) -> SqlResult<SqlSchema> {
        Ok(sql_schema_describer::sqlite::SqlSchemaDescriber::new(conn)
            .describe(schema_name)
            .await?)
    }

    async fn initialize(&self, _conn: &dyn Queryable, _database_info: &DatabaseInfo) -> SqlResult<()> {
        let path_buf = PathBuf::from(&self.file_path);

        if let Some(parent_directory) = path_buf.parent() {
            fs::create_dir_all(parent_directory).expect("creating the database folders failed")
        }

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Sqlite
    }
}

#[derive(Debug)]
pub(crate) struct PostgresFlavour(pub(crate) PostgresUrl);

#[async_trait::async_trait]
impl SqlFlavour for PostgresFlavour {
    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        let url = Url::parse(database_str).unwrap();
        let db_name = self.0.dbname();

        let (conn, _) = create_postgres_admin_conn(url).await?;

        let query = format!("CREATE DATABASE \"{}\"", db_name);
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from)).await?;

        Ok(db_name.to_owned())
    }

    async fn describe_schema<'a>(
        &'a self,
        schema_name: &'a str,
        conn: Arc<dyn Queryable + Send + Sync>,
    ) -> SqlResult<SqlSchema> {
        Ok(sql_schema_describer::postgres::SqlSchemaDescriber::new(conn)
            .describe(schema_name)
            .await?)
    }

    async fn initialize(&self, conn: &dyn Queryable, database_info: &DatabaseInfo) -> SqlResult<()> {
        let schema_sql = format!(
            "CREATE SCHEMA IF NOT EXISTS \"{}\";",
            &database_info.connection_info().schema_name()
        );

        conn.raw_cmd(&schema_sql).await?;

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Postgres
    }
}

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
async fn create_postgres_admin_conn(mut url: Url) -> ConnectorResult<(Quaint, DatabaseInfo)> {
    use migration_connector::ErrorKind;

    let candidate_default_databases = &["postgres", "template1"];

    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params: Vec<String> = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let params: String = params.join("&");
    url.set_query(Some(&params));

    let mut conn = None;

    for database_name in candidate_default_databases {
        url.set_path(&format!("/{}", database_name));
        match connect(url.as_str()).await {
            // If the database does not exist, try the next one.
            Err(err) => match &err.kind {
                migration_connector::ErrorKind::DatabaseDoesNotExist { .. } => (),
                _other_outcome => {
                    conn = Some(Err(err));
                    break;
                }
            },
            // If the outcome is anything else, use this.
            other_outcome => {
                conn = Some(other_outcome);
                break;
            }
        }
    }

    let conn = conn
        .ok_or_else(|| {
            ConnectorError::from_kind(ErrorKind::DatabaseCreationFailed {
                explanation: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned()
            })
        })??;

    Ok(conn)
}
