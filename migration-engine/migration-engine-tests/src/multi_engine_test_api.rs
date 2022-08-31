#![deny(missing_docs)]

//! A TestApi that is initialized without IO or async code and can instantiate
//! multiple migration engines.

pub use test_macros::test_connector;
pub use test_setup::sqlite_test_url;
pub use test_setup::{runtime::run_with_thread_local_runtime as tok, BitFlags, Capabilities, Tags};

use crate::{
    assertions::SchemaAssertion,
    commands::{ApplyMigrations, CreateMigration, DiagnoseMigrationHistory, Reset, SchemaPush},
};
use migration_core::migration_connector::{ConnectorParams, MigrationConnector};
use psl::common::preview_features::PreviewFeature;
use quaint::{
    prelude::{ConnectionInfo, Queryable, ResultSet},
    single::Quaint,
};
use sql_migration_connector::SqlMigrationConnector;
use tempfile::TempDir;
use test_setup::{DatasourceBlock, TestApiArgs};

/// The multi-engine test API.
pub struct TestApi {
    pub(crate) args: TestApiArgs,
    connection_string: String,
    pub(crate) admin_conn: Quaint,
    preview_features: BitFlags<PreviewFeature>,
}

impl TestApi {
    /// Initializer, called by the test macros.
    pub fn new(args: TestApiArgs) -> Self {
        let tags = args.tags();

        let preview_features = args
            .preview_features()
            .iter()
            .flat_map(|f| PreviewFeature::parse_opt(f))
            .collect();

        let (admin_conn, connection_string) = if tags.contains(Tags::Postgres) {
            let (_, q, cs) = tok(args.create_postgres_database());
            (q, cs)
        } else if tags.contains(Tags::Vitess) {
            let params = ConnectorParams {
                connection_string: args.database_url().to_owned(),
                preview_features,
                shadow_database_connection_string: args.shadow_database_url().map(String::from),
            };
            let mut conn = SqlMigrationConnector::new_mysql();
            conn.set_params(params).unwrap();
            tok(conn.reset(false)).unwrap();

            (
                tok(Quaint::new(args.database_url())).unwrap(),
                args.database_url().to_owned(),
            )
        } else if tags.contains(Tags::Mysql) {
            let (_, cs) = tok(args.create_mysql_database());
            (tok(Quaint::new(&cs)).unwrap(), cs)
        } else if tags.contains(Tags::Mssql) {
            tok(args.create_mssql_database())
        } else if tags.contains(Tags::Sqlite) {
            let url = test_setup::sqlite_test_url(args.test_function_name());

            (tok(Quaint::new(&url)).unwrap(), url)
        } else {
            unreachable!()
        };

        TestApi {
            args,
            connection_string,
            admin_conn,
            preview_features,
        }
    }

    /// Equivalent to quaint's query_raw()
    pub fn query_raw(&self, sql: &str, params: &[quaint::Value<'_>]) -> quaint::Result<ResultSet> {
        tok(self.admin_conn.query_raw(sql, params))
    }

    /// Send a SQL command to the database, and expect it to succeed.
    pub fn raw_cmd(&self, sql: &str) {
        tok(self.admin_conn.raw_cmd(sql)).unwrap()
    }

    /// The connection string for the database associated with the test.
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// The ConnectionInfo based on the connection string
    pub fn connection_info(&self) -> ConnectionInfo {
        ConnectionInfo::from_url(self.connection_string()).unwrap()
    }

    /// Create a temporary directory to serve as a test migrations directory.
    pub fn create_migrations_directory(&self) -> TempDir {
        tempfile::tempdir().unwrap()
    }

    /// Render a valid datasource block, including database URL.
    pub fn datasource_block(&self) -> DatasourceBlock<'_> {
        self.args.datasource_block(self.args.database_url(), &[])
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
        self.tags().contains(Tags::Mysql8)
    }

    /// Returns true only when testing on postgres.
    pub fn is_postgres(&self) -> bool {
        self.tags().contains(Tags::Postgres)
    }

    /// Returns true only when testing on postgres version 15.
    pub fn is_postgres_15(&self) -> bool {
        self.tags().contains(Tags::Postgres15)
    }

    /// Returns true only when testing on cockroach.
    pub fn is_cockroach(&self) -> bool {
        self.tags().contains(Tags::CockroachDb)
    }

    /// Returns true only when testing on sqlite.
    pub fn is_sqlite(&self) -> bool {
        self.tags().contains(Tags::Sqlite)
    }

    /// Returns true only when testing on vitess.
    pub fn is_vitess(&self) -> bool {
        self.tags().contains(Tags::Vitess)
    }

    /// Returns whether the database automatically lower-cases table names.
    pub fn lower_cases_table_names(&self) -> bool {
        self.tags().contains(Tags::LowerCasesTableNames)
    }

    /// Instantiate a new migration engine for the current database.
    pub fn new_engine(&self) -> EngineTestApi {
        let shadow_db = self.args.shadow_database_url().as_ref().map(ToString::to_string);
        self.new_engine_with_connection_strings(self.connection_string.clone(), shadow_db)
    }

    /// Instantiate a new migration with the provided connection string.
    pub fn new_engine_with_connection_strings(
        &self,
        connection_string: String,
        shadow_database_connection_string: Option<String>,
    ) -> EngineTestApi {
        let connection_info = ConnectionInfo::from_url(&connection_string).unwrap();

        let params = ConnectorParams {
            connection_string,
            preview_features: self.preview_features,
            shadow_database_connection_string,
        };

        let mut connector = match &connection_info {
            ConnectionInfo::Postgres(_) => {
                if self.args.provider() == "cockroachdb" {
                    SqlMigrationConnector::new_cockroach()
                } else {
                    SqlMigrationConnector::new_postgres()
                }
            }
            ConnectionInfo::Mysql(_) => SqlMigrationConnector::new_mysql(),
            ConnectionInfo::Mssql(_) => SqlMigrationConnector::new_mssql(),
            ConnectionInfo::Sqlite { .. } => SqlMigrationConnector::new_sqlite(),
            ConnectionInfo::InMemorySqlite { .. } => unreachable!(),
        };
        connector.set_params(params).unwrap();

        EngineTestApi {
            connector,
            connection_info,
            tags: self.args.tags(),
        }
    }

    fn tags(&self) -> BitFlags<Tags> {
        self.args.tags()
    }

    /// The name of the test function, as a string.
    pub fn test_fn_name(&self) -> &str {
        self.args.test_function_name()
    }

    /// Render a datamodel including provider and generator block.
    pub fn datamodel_with_provider(&self, schema: &str) -> String {
        let mut out = String::with_capacity(320 + schema.len());

        self.write_datasource_block(&mut out);
        out.push_str(&self.generator_block());
        out.push_str(schema);

        out
    }

    /// Render a valid datasource block, including database URL.
    pub fn write_datasource_block(&self, out: &mut dyn std::fmt::Write) {
        write!(out, "{}", self.args.datasource_block(self.args.database_url(), &[])).unwrap()
    }

    /// Currently enabled preview features.
    pub fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.preview_features
    }

    fn generator_block(&self) -> String {
        let preview_features: Vec<String> = self
            .args
            .preview_features()
            .iter()
            .map(|pf| format!(r#""{}""#, pf))
            .collect();

        let preview_feature_string = if preview_features.is_empty() {
            "".to_string()
        } else {
            format!("\npreviewFeatures = [{}]", preview_features.join(", "))
        };

        let generator_block = format!(
            r#"generator client {{
                 provider = "prisma-client-js"{}
               }}"#,
            preview_feature_string
        );
        generator_block
    }
}

/// A wrapper around a migration engine instance optimized for convenience in
/// writing tests.
pub struct EngineTestApi {
    pub(crate) connector: SqlMigrationConnector,
    connection_info: ConnectionInfo,
    tags: BitFlags<Tags>,
}

impl EngineTestApi {
    /// Plan an `applyMigrations` command
    pub fn apply_migrations<'a>(&'a mut self, migrations_directory: &'a TempDir) -> ApplyMigrations<'a> {
        ApplyMigrations::new(&mut self.connector, migrations_directory)
    }

    /// Plan a `createMigration` command
    pub fn create_migration<'a>(
        &'a mut self,
        name: &'a str,
        schema: &'a str,
        migrations_directory: &'a TempDir,
    ) -> CreateMigration<'a> {
        CreateMigration::new(&mut self.connector, name, schema, migrations_directory)
    }

    /// Builder and assertions to call the DiagnoseMigrationHistory command.
    pub fn diagnose_migration_history<'a>(
        &'a mut self,
        migrations_directory: &'a TempDir,
    ) -> DiagnoseMigrationHistory<'a> {
        DiagnoseMigrationHistory::new(&mut self.connector, migrations_directory)
    }

    /// Assert facts about the database schema
    pub fn assert_schema(&mut self) -> SchemaAssertion {
        SchemaAssertion::new(tok(self.connector.describe_schema()).unwrap(), self.tags)
    }

    /// Plan a `reset` command
    pub fn reset(&mut self) -> Reset<'_> {
        Reset::new(&mut self.connector)
    }

    /// Plan a `schemaPush` command
    pub fn schema_push(&mut self, dm: impl Into<String>) -> SchemaPush<'_> {
        SchemaPush::new(&mut self.connector, dm.into())
    }

    /// The schema name of the current connected database.
    pub fn schema_name(&self) -> &str {
        self.connection_info.schema_name()
    }

    /// Execute a raw SQL command and expect it to succeed.
    #[track_caller]
    pub fn raw_cmd(&mut self, cmd: &str) {
        tok(self.connector.raw_cmd(cmd)).unwrap()
    }
}
