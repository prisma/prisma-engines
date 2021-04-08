#![deny(missing_docs)]

//! A TestApi that is initialized without IO or async code and can instantiate
//! multiple migration engines.

use crate::{ApplyMigrations, CreateMigration, DiagnoseMigrationHistory, Reset, SchemaAssertion, SchemaPush};
use enumflags2::BitFlags;
use migration_core::GenericApi;
use quaint::{prelude::Queryable, single::Quaint};
use sql_migration_connector::SqlMigrationConnector;
use tempfile::TempDir;
use test_setup::{connectors::Tags, TestApiArgs};

/// The multi-engine test API.
pub struct TestApi {
    args: TestApiArgs,
    connection_string: String,
}

impl TestApi {
    /// Initializer, called by the test macros.
    pub fn new(args: TestApiArgs) -> Self {
        let connection_string = (args.url_fn)(args.test_function_name);

        TestApi {
            args,
            connection_string,
        }
    }

    /// The connection string for the database associated with the test.
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// Create a temporary directory to serve as a test migrations directory.
    pub fn create_migrations_directory(&self) -> anyhow::Result<TempDir> {
        Ok(tempfile::tempdir()?)
    }

    /// Returns true only when testing on MSSQL.
    pub fn is_mssql(&self) -> bool {
        self.args.connector_tags.contains(Tags::Mssql)
    }

    /// Returns true only when testing on MySQL.
    pub fn is_mysql(&self) -> bool {
        self.args.connector_tags.contains(Tags::Mysql)
    }

    /// Returns true only when testing on MariaDB.
    pub fn is_mysql_mariadb(&self) -> bool {
        self.args.connector_tags.contains(Tags::Mariadb)
    }

    /// Returns true only when testing on MySQL 5.6.
    pub fn is_mysql_5_6(&self) -> bool {
        self.args.connector_tags.contains(Tags::Mysql56)
    }

    /// Returns true only when testing on MySQL 8.
    pub fn is_mysql_8(&self) -> bool {
        self.args.connector_tags.contains(Tags::Mysql8)
    }

    /// Returns true only when testing on postgres.
    pub fn is_postgres(&self) -> bool {
        self.args.connector_tags.contains(Tags::Postgres)
    }

    /// Instantiate a new migration engine for the current database.
    pub async fn new_engine(&self) -> anyhow::Result<EngineTestApi> {
        self.new_engine_with_connection_strings(&self.connection_string, None)
            .await
    }

    /// Instantiate a new migration with the provided connection string.
    pub async fn new_engine_with_connection_strings(
        &self,
        connection_string: &str,
        shadow_db_connection_string: Option<String>,
    ) -> anyhow::Result<EngineTestApi> {
        let connector =
            SqlMigrationConnector::new(&connection_string, BitFlags::empty(), shadow_db_connection_string).await?;

        Ok(EngineTestApi(connector))
    }

    /// Initialize the database.
    pub async fn initialize(&self) -> anyhow::Result<Quaint> {
        if self.args.connector_tags.contains(Tags::Postgres) {
            Ok(test_setup::create_postgres_database(&self.connection_string.parse()?)
                .await
                .unwrap())
        } else if self.args.connector_tags.contains(Tags::Mysql) {
            Ok(test_setup::create_mysql_database(&self.connection_string.parse()?)
                .await
                .unwrap())
        } else if self.args.connector_tags.contains(Tags::Mssql) {
            let conn = Quaint::new(&self.connection_string).await?;
            test_setup::connectors::mssql::reset_schema(&conn, self.args.test_function_name).await?;
            Ok(conn)
        } else {
            Ok(Quaint::new(&self.connection_string).await?)
        }
    }

    /// The name of the test function, as a string.
    pub fn test_fn_name(&self) -> &str {
        self.args.test_function_name
    }
}

/// A wrapper around a migration engine instance optimized for convenience in
/// writing tests.
pub struct EngineTestApi(SqlMigrationConnector);

impl EngineTestApi {
    /// Plan an `applyMigrations` command
    pub fn apply_migrations<'a>(&'a self, migrations_directory: &'a TempDir) -> ApplyMigrations<'a> {
        ApplyMigrations::new(&self.0, migrations_directory)
    }

    /// Plan a `createMigration` command
    pub fn create_migration<'a>(
        &'a self,
        name: &'a str,
        schema: &'a str,
        migrations_directory: &'a TempDir,
    ) -> CreateMigration<'a> {
        CreateMigration::new(&self.0, name, schema, migrations_directory)
    }

    /// Builder and assertions to call the DiagnoseMigrationHistory command.
    pub fn diagnose_migration_history<'a>(&'a self, migrations_directory: &'a TempDir) -> DiagnoseMigrationHistory<'a> {
        DiagnoseMigrationHistory::new(&self.0, migrations_directory)
    }

    /// Assert facts about the database schema
    pub async fn assert_schema(&self) -> Result<SchemaAssertion, anyhow::Error> {
        let schema = self.0.describe_schema().await?;

        Ok(SchemaAssertion::new(schema, BitFlags::empty()))
    }

    /// True if MySQL on Windows with default settings.
    pub async fn lower_case_identifiers(&self) -> bool {
        self.0
            .quaint()
            .query_raw("SELECT @@lower_case_table_names", &[])
            .await
            .ok()
            .and_then(|row| row.into_single().ok())
            .and_then(|row| row.at(0).and_then(|col| col.as_i64()))
            .map(|val| val == 1)
            .unwrap_or(false)
    }

    /// Expose the GenericApi impl.
    pub fn generic_api(&self) -> &dyn GenericApi {
        &self.0
    }

    /// Plan a `reset` command
    pub fn reset(&self) -> Reset<'_> {
        Reset::new(&self.0)
    }

    /// Plan a `schemaPush` command
    pub fn schema_push(&self, dm: impl Into<String>) -> SchemaPush<'_> {
        SchemaPush::new(&self.0, dm.into())
    }

    /// The schema name of the current connected database.
    pub fn schema_name(&self) -> &str {
        self.0.quaint().connection_info().schema_name()
    }

    /// Execute a raw SQL command.
    pub async fn raw_cmd(&self, cmd: &str) -> Result<(), quaint::error::Error> {
        self.0.quaint().raw_cmd(cmd).await
    }
}
