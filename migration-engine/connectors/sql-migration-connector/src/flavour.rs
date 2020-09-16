#![deny(missing_docs)]

//! SQL flavours implement behaviour specific to a given SQL implementation (PostgreSQL, SQLite...),
//! in order to avoid cluttering the connector with conditionals. This is a private implementation
//! detail of the SQL connector.

use crate::{
    catch, connect,
    database_info::DatabaseInfo,
    error::{CheckDatabaseInfoResult, SystemDatabase},
    sql_destructive_change_checker::DestructiveChangeCheckerFlavour,
    sql_renderer::SqlRenderer,
    sql_schema_calculator::SqlSchemaCalculatorFlavour,
    sql_schema_differ::SqlSchemaDifferFlavour,
    SqlError, SqlResult,
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
use std::{collections::HashMap, fmt::Debug, path::Path, sync::Arc};
use url::Url;

/// The maximum size of identifiers on MySQL, in bytes.
///
/// reference: https://dev.mysql.com/doc/refman/5.7/en/identifier-length.html
pub(crate) const MYSQL_IDENTIFIER_SIZE_LIMIT: usize = 64;

pub(crate) fn from_connection_info(connection_info: &ConnectionInfo) -> Box<dyn SqlFlavour + Send + Sync + 'static> {
    match connection_info {
        ConnectionInfo::Mysql(url) => Box::new(MysqlFlavour(url.clone())),
        ConnectionInfo::Postgres(url) => Box::new(PostgresFlavour(url.clone())),
        ConnectionInfo::Sqlite { file_path, db_name } => Box::new(SqliteFlavour {
            file_path: file_path.clone(),
            attached_name: db_name.clone(),
        }),
        ConnectionInfo::Mssql(_) => todo!("Greetings from Redmond!"),
    }
}

#[async_trait::async_trait]
pub(crate) trait SqlFlavour:
    DestructiveChangeCheckerFlavour + SqlRenderer + SqlSchemaDifferFlavour + SqlSchemaCalculatorFlavour + Debug
{
    /// This method should be considered deprecated. Prefer extending SqlFlavour
    /// with methods expressing clearly what is being specialized by database
    /// backend.
    fn sql_family(&self) -> SqlFamily;

    /// Optionally validate the database info.
    fn check_database_info(&self, _database_info: &DatabaseInfo) -> CheckDatabaseInfoResult {
        Ok(())
    }

    /// Check a connection to make sure it is usable by the migration engine.
    /// This can include some set up on the database, like ensuring that the
    /// schema we connect to exists.
    async fn ensure_connection_validity(&self, connection: &Quaint) -> ConnectorResult<()>;

    /// Make sure that the `_prisma_migrations` table exists.
    async fn ensure_imperative_migrations_table(
        &self,
        connection: &dyn Queryable,
        connection_info: &ConnectionInfo,
    ) -> ConnectorResult<()>;

    /// Create a database for the given URL on the server, if applicable.
    async fn create_database(&self, database_url: &str) -> ConnectorResult<String>;

    /// Perform the initialization required by connector-test-kit tests.
    async fn qe_setup(&self, database_url: &str) -> ConnectorResult<()>;

    /// Introspect the SQL schema.
    async fn describe_schema<'a>(
        &'a self,
        schema_name: &'a str,
        conn: Arc<dyn Queryable + Send + Sync>,
    ) -> SqlResult<SqlSchema>;
}

#[derive(Debug)]
pub(crate) struct MysqlFlavour(MysqlUrl);

impl MysqlFlavour {
    pub(crate) fn schema_name(&self) -> &str {
        self.0.dbname()
    }
}

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
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
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

    async fn ensure_connection_validity(&self, connection: &Quaint) -> ConnectorResult<()> {
        catch(
            connection.connection_info(),
            connection.raw_cmd("SELECT 1").map_err(SqlError::from),
        )
        .await?;

        Ok(())
    }

    async fn ensure_imperative_migrations_table(
        &self,
        connection: &dyn Queryable,
        connection_info: &ConnectionInfo,
    ) -> ConnectorResult<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS _prisma_migrations (
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             DATETIME(3),
                migration_name          TEXT NOT NULL,
                logs                    TEXT NOT NULL,
                rolled_back_at          DATETIME(3),
                started_at              DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
                applied_steps_count     INTEGER UNSIGNED NOT NULL DEFAULT 0,
                script                  TEXT NOT NULL
            ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
        "#;

        catch(connection_info, connection.raw_cmd(sql).map_err(SqlError::from)).await
    }

    async fn qe_setup(&self, database_str: &str) -> ConnectorResult<()> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        url.set_path("/mysql");

        let (conn, _) = connect(&url.to_string()).await?;

        let db_name = self.0.dbname();

        let query = format!("DROP DATABASE IF EXISTS `{}`", db_name);
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from)).await?;

        let query = format!(
            "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;",
            db_name
        );
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from)).await?;

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Mysql
    }
}

#[derive(Debug)]
pub(crate) struct SqliteFlavour {
    file_path: String,
    attached_name: String,
}

impl SqliteFlavour {
    pub(crate) fn attached_name(&self) -> &str {
        &self.attached_name
    }
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

    async fn ensure_connection_validity(&self, _connection: &Quaint) -> ConnectorResult<()> {
        Ok(())
    }

    async fn ensure_imperative_migrations_table(
        &self,
        connection: &dyn Queryable,
        connection_info: &ConnectionInfo,
    ) -> ConnectorResult<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS "_prisma_migrations" (
                "id"                    TEXT PRIMARY KEY NOT NULL,
                "checksum"              TEXT NOT NULL,
                "finished_at"           DATETIME,
                "migration_name"        TEXT NOT NULL,
                "logs"                  TEXT NOT NULL,
                "rolled_back_at"        DATETIME,
                "started_at"            DATETIME NOT NULL DEFAULT current_timestamp,
                "applied_steps_count"   INTEGER UNSIGNED NOT NULL DEFAULT 0,
                "script"                TEXT NOT NULL
            );
        "#;

        catch(connection_info, connection.raw_cmd(sql).map_err(SqlError::from)).await
    }

    async fn qe_setup(&self, _database_url: &str) -> ConnectorResult<()> {
        use std::fs::File;
        File::create(&self.file_path).expect("Failed to truncate SQLite database");
        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Sqlite
    }
}

#[derive(Debug)]
pub(crate) struct PostgresFlavour(pub(crate) PostgresUrl);

impl PostgresFlavour {
    pub(crate) fn schema_name(&self) -> &str {
        self.0.schema()
    }
}

#[async_trait::async_trait]
impl SqlFlavour for PostgresFlavour {
    async fn create_database(&self, database_str: &str) -> ConnectorResult<String> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let db_name = self.0.dbname();

        strip_schema_param_from_url(&mut url);

        let (conn, _) = create_postgres_admin_conn(url.clone()).await?;

        let query = format!("CREATE DATABASE \"{}\"", db_name);

        let mut database_already_exists_error = None;

        match conn.raw_cmd(&query).map_err(SqlError::from).await {
            Ok(_) => (),
            Err(err @ SqlError::DatabaseAlreadyExists { .. }) => database_already_exists_error = Some(err),
            Err(err @ SqlError::UniqueConstraintViolation { .. }) => database_already_exists_error = Some(err),
            Err(err) => return Err(SqlError::from(err).into_connector_error(conn.connection_info())),
        };

        // Now create the schema
        url.set_path(&format!("/{}", db_name));

        let (conn, _) = connect(&url.to_string()).await?;

        let schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", &self.schema_name());

        catch(
            conn.connection_info(),
            conn.raw_cmd(&schema_sql).map_err(SqlError::from),
        )
        .await?;

        if let Some(err) = database_already_exists_error {
            return Err(err.into_connector_error(conn.connection_info()));
        }

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

    async fn ensure_connection_validity(&self, connection: &Quaint) -> ConnectorResult<()> {
        let schema_exists_result = catch(
            connection.connection_info(),
            connection
                .query_raw(
                    "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname = $1)",
                    &[connection.connection_info().schema_name().into()],
                )
                .map_err(SqlError::from),
        )
        .await?;

        if let Some(true) = schema_exists_result
            .get(0)
            .and_then(|row| row.at(0).and_then(|value| value.as_bool()))
        {
            return Ok(());
        }

        tracing::debug!(
            "Detected that the `{schema_name}` schema does not exist on the target database. Attempting to create it.",
            schema_name = connection.connection_info().schema_name(),
        );

        catch(
            connection.connection_info(),
            connection
                .raw_cmd(&format!(
                    "CREATE SCHEMA \"{}\"",
                    connection.connection_info().schema_name()
                ))
                .map_err(SqlError::from),
        )
        .await?;

        Ok(())
    }

    async fn ensure_imperative_migrations_table(
        &self,
        connection: &dyn Queryable,
        connection_info: &ConnectionInfo,
    ) -> ConnectorResult<()> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS _prisma_migrations (
                id                      VARCHAR(36) PRIMARY KEY NOT NULL,
                checksum                VARCHAR(64) NOT NULL,
                finished_at             TIMESTAMPTZ,
                migration_name          TEXT NOT NULL,
                logs                    TEXT NOT NULL,
                rolled_back_at          TIMESTAMPTZ,
                started_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
                applied_steps_count     INTEGER NOT NULL DEFAULT 0,
                script                  TEXT NOT NULL
            );
        "#;

        catch(connection_info, connection.raw_cmd(sql).map_err(SqlError::from)).await
    }

    async fn qe_setup(&self, database_str: &str) -> ConnectorResult<()> {
        let mut url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

        strip_schema_param_from_url(&mut url);
        let (conn, _) = create_postgres_admin_conn(url.clone()).await?;
        let schema = self.0.schema();
        let db_name = self.0.dbname();

        let query = format!("CREATE DATABASE \"{}\"", db_name);
        catch(conn.connection_info(), conn.raw_cmd(&query).map_err(SqlError::from))
            .await
            .ok();

        // Now create the schema
        url.set_path(&format!("/{}", db_name));

        let (conn, _) = connect(&url.to_string()).await?;

        let drop_and_recreate_schema = format!(
            "DROP SCHEMA IF EXISTS \"{schema}\" CASCADE;\nCREATE SCHEMA \"{schema}\";",
            schema = schema
        );
        catch(
            conn.connection_info(),
            conn.raw_cmd(&drop_and_recreate_schema).map_err(SqlError::from),
        )
        .await?;

        Ok(())
    }

    fn sql_family(&self) -> SqlFamily {
        SqlFamily::Postgres
    }
}

fn strip_schema_param_from_url(url: &mut Url) {
    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params: Vec<String> = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let params: String = params.join("&");
    url.set_query(Some(&params));
}

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
async fn create_postgres_admin_conn(mut url: Url) -> ConnectorResult<(Quaint, DatabaseInfo)> {
    use migration_connector::ErrorKind;

    let candidate_default_databases = &["postgres", "template1"];

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
