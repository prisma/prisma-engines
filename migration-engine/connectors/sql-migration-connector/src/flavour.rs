//! SQL flavours implement behaviour specific to a given SQL implementation (PostgreSQL, SQLite...),
//! in order to avoid cluttering the connector with conditionals. This is a private implementation
//! detail of the SQL connector.

mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use enumflags2::BitFlags;
pub(crate) use mssql::MssqlFlavour;
pub(crate) use mysql::MysqlFlavour;
pub(crate) use postgres::PostgresFlavour;
pub(crate) use sqlite::SqliteFlavour;
use user_facing_errors::migration_engine::ApplyMigrationError;

use crate::{
    connection_wrapper::Connection, sql_destructive_change_checker::DestructiveChangeCheckerFlavour,
    sql_renderer::SqlRenderer, sql_schema_calculator::SqlSchemaCalculatorFlavour,
    sql_schema_differ::SqlSchemaDifferFlavour, SqlMigrationConnector,
};
use datamodel::{common::preview_features::PreviewFeature, Datamodel};
use migration_connector::{migrations_directory::MigrationDirectory, ConnectorError, ConnectorResult};
use quaint::prelude::{ConnectionInfo, Table};
use sql_schema_describer::SqlSchema;
use std::fmt::Debug;

/// The maximum size of identifiers on MySQL, in bytes.
///
/// reference: https://dev.mysql.com/doc/refman/5.7/en/identifier-length.html
pub(crate) const MYSQL_IDENTIFIER_SIZE_LIMIT: usize = 64;

pub(crate) fn from_connection_info(
    connection_info: &ConnectionInfo,
    preview_features: BitFlags<PreviewFeature>,
) -> Box<dyn SqlFlavour + Send + Sync + 'static> {
    match connection_info {
        ConnectionInfo::Mysql(url) => Box::new(MysqlFlavour::new(url.clone(), preview_features)),
        ConnectionInfo::Postgres(url) => Box::new(PostgresFlavour::new(url.clone(), preview_features)),
        ConnectionInfo::Sqlite { file_path, db_name } => Box::new(SqliteFlavour {
            file_path: file_path.clone(),
            attached_name: db_name.clone(),
            preview_features,
        }),
        ConnectionInfo::Mssql(url) => Box::new(MssqlFlavour::new(url.clone(), preview_features)),
        ConnectionInfo::InMemorySqlite { .. } => unreachable!("SqlFlavour for in-memory SQLite"),
    }
}

#[async_trait::async_trait]
pub(crate) trait SqlFlavour:
    DestructiveChangeCheckerFlavour + SqlRenderer + SqlSchemaDifferFlavour + SqlSchemaCalculatorFlavour + Debug
{
    async fn acquire_lock(&self, connection: &Connection) -> ConnectorResult<()>;

    async fn apply_migration_script(
        &self,
        migration_name: &str,
        script: &str,
        conn: &Connection,
    ) -> ConnectorResult<()>;

    fn check_database_version_compatibility(
        &self,
        _datamodel: &Datamodel,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        None
    }

    /// Create a database for the given URL on the server, if applicable.
    async fn create_database(&self, database_url: &str) -> ConnectorResult<String>;

    /// Initialize the `_prisma_migrations` table.
    async fn create_migrations_table(&self, connection: &Connection) -> ConnectorResult<()>;

    /// Describe the SQL schema.
    async fn describe_schema<'a>(&'a self, conn: &Connection) -> ConnectorResult<SqlSchema>;

    /// Drop the database for the provided URL on the server.
    async fn drop_database(&self, database_url: &str) -> ConnectorResult<()>;

    /// Drop the migrations table
    async fn drop_migrations_table(&self, connection: &Connection) -> ConnectorResult<()>;

    /// Check a connection to make sure it is usable by the migration engine.
    /// This can include some set up on the database, like ensuring that the
    /// schema we connect to exists.
    async fn ensure_connection_validity(&self, connection: &Connection) -> ConnectorResult<()>;

    /// Perform the initialization required by connector-test-kit tests.
    async fn qe_setup(&self, database_url: &str) -> ConnectorResult<()>;

    /// Drop the database and recreate it empty.
    async fn reset(&self, connection: &Connection) -> ConnectorResult<()>;

    /// Optionally scan a migration script that could have been altered by users and emit warnings.
    fn scan_migration_script(&self, _script: &str) {}

    /// Apply the given migration history to a shadow database, and return
    /// the final introspected SQL schema.
    async fn sql_schema_from_migration_history(
        &self,
        migrations: &[MigrationDirectory],
        connection: &Connection,
        connector: &SqlMigrationConnector,
    ) -> ConnectorResult<SqlSchema>;

    /// Runs a single SQL script.
    async fn run_query_script(&self, sql: &str, connection: &Connection) -> ConnectorResult<()>;

    /// The preview features in use.
    fn preview_features(&self) -> BitFlags<PreviewFeature>;

    /// Table to store applied migrations, the name part.
    fn migrations_table_name(&self) -> &'static str {
        "_prisma_migrations"
    }

    /// Table to store applied migrations.
    fn migrations_table(&self) -> Table<'_> {
        self.migrations_table_name().into()
    }
}

// Utility function shared by multiple flavours to compare shadow database and main connection.
fn validate_connection_infos_do_not_match((previous, next): (&ConnectionInfo, &ConnectionInfo)) -> ConnectorResult<()> {
    if previous.host() == next.host() && previous.dbname() == next.dbname() && previous.port() == next.port() {
        Err(ConnectorError::from_msg("The shadow database you configured appears to be the same as the main database. Please specify another shadow database.".into()))
    } else {
        Ok(())
    }
}

async fn generic_apply_migration_script(migration_name: &str, script: &str, conn: &Connection) -> ConnectorResult<()> {
    conn.raw_cmd(script).await.map_err(|quaint_error| {
        ConnectorError::user_facing(ApplyMigrationError {
            migration_name: migration_name.to_owned(),
            database_error_code: String::from(quaint_error.original_code().unwrap_or("none")),
            database_error: quaint_error
                .original_message()
                .map(String::from)
                .unwrap_or_else(|| ConnectorError::from(quaint_error).to_string()),
        })
    })
}
