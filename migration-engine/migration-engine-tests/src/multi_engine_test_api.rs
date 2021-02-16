#![deny(missing_docs)]

//! A TestApi that is initialized without IO or async code and can instantiate
//! multiple migration engines.

use crate::{ApplyMigrations, CreateMigration, Reset, SchemaAssertion, SchemaPush};
use enumflags2::BitFlags;
use migration_core::GenericApi;
use quaint::single::Quaint;
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
    pub async fn new(args: TestApiArgs) -> Self {
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

    /// Assert facts about the database schema
    pub async fn assert_schema(&self) -> Result<SchemaAssertion, anyhow::Error> {
        let schema = self.0.describe_schema().await?;

        Ok(SchemaAssertion(schema))
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
}
