#![deny(missing_docs)]

//! A TestApi that is initialized without IO or async code and can instantiate
//! multiple migration engines.

pub use test_setup::{BitFlags, Capabilities, Tags};

use crate::{ApplyMigrations, CreateMigration, DiagnoseMigrationHistory, Reset, SchemaAssertion, SchemaPush};
use migration_core::GenericApi;
use quaint::{prelude::Queryable, single::Quaint};
use sql_migration_connector::SqlMigrationConnector;
use tempfile::TempDir;
use test_setup::TestApiArgs;

/// The multi-engine test API.
pub struct TestApi {
    args: TestApiArgs,
    connection_string: String,
    admin_conn: Quaint,
}

impl TestApi {
    /// Initializer, called by the test macros.
    pub async fn new(args: TestApiArgs) -> Self {
        let tags = args.tags();
        let db_name = args.test_function_name();

        let (admin_conn, connection_string) = if tags.contains(Tags::Postgres) {
            test_setup::create_postgres_database(db_name).await.unwrap()
        } else if tags.contains(Tags::Vitess) {
            SqlMigrationConnector::new(args.database_url(), args.shadow_database_url().map(String::from))
                .await
                .unwrap()
                .reset()
                .await
                .unwrap();

            (
                Quaint::new(args.database_url()).await.unwrap(),
                args.database_url().to_owned(),
            )
        } else if tags.contains(Tags::Mysql) {
            test_setup::create_mysql_database(db_name).await.unwrap()
        } else if tags.contains(Tags::Mssql) {
            test_setup::init_mssql_database(args.database_url(), db_name)
                .await
                .unwrap()
        } else if tags.contains(Tags::Sqlite) {
            let url = test_setup::sqlite_test_url(db_name);

            (Quaint::new(&url).await.unwrap(), url)
        } else {
            unreachable!()
        };

        TestApi {
            args,
            admin_conn,
            connection_string,
        }
    }

    /// The default connection to the database.
    pub fn admin_conn(&self) -> &Quaint {
        &self.admin_conn
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
        self.tags().contains(Tags::Mssql)
    }

    /// Returns true only when testing on MySQL.
    pub fn is_mysql(&self) -> bool {
        self.tags().contains(Tags::Mysql)
    }

    /// Returns true only when testing on MariaDB.
    pub fn is_mysql_mariadb(&self) -> bool {
        self.tags().contains(Tags::Mariadb)
    }

    /// Returns true only when testing on MySQL 5.6.
    pub fn is_mysql_5_6(&self) -> bool {
        self.tags().contains(Tags::Mysql56)
    }

    /// Returns true only when testing on MySQL 8.
    pub fn is_mysql_8(&self) -> bool {
        self.tags().intersects(Tags::Mysql8 | Tags::Vitess80)
    }

    /// Returns true only when testing on postgres.
    pub fn is_postgres(&self) -> bool {
        self.tags().contains(Tags::Postgres)
    }

    /// Returns true only when testing on vitess.
    pub fn is_vitess(&self) -> bool {
        self.tags().contains(Tags::Vitess)
    }

    /// Instantiate a new migration engine for the current database.
    pub async fn new_engine(&self) -> anyhow::Result<EngineTestApi> {
        let shadow_db = self.args.shadow_database_url().as_ref().map(ToString::to_string);

        self.new_engine_with_connection_strings(&self.connection_string, shadow_db)
            .await
    }

    /// Instantiate a new migration with the provided connection string.
    pub async fn new_engine_with_connection_strings(
        &self,
        connection_string: &str,
        shadow_db_connection_string: Option<String>,
    ) -> anyhow::Result<EngineTestApi> {
        let connector = SqlMigrationConnector::new(&connection_string, shadow_db_connection_string).await?;

        Ok(EngineTestApi(connector, self.args.tags()))
    }

    fn tags(&self) -> BitFlags<Tags> {
        self.args.tags()
    }

    /// The name of the test function, as a string.
    pub fn test_fn_name(&self) -> &str {
        self.args.test_function_name()
    }
}

/// A wrapper around a migration engine instance optimized for convenience in
/// writing tests.
pub struct EngineTestApi(SqlMigrationConnector, BitFlags<Tags>);

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

        Ok(SchemaAssertion::new(schema, self.1))
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
