use migration_connector::MigrationPersistence;
use tempfile::TempDir;
pub use test_macros::test_connector;
pub use test_setup::{BitFlags, Capabilities, Tags};

use crate::{
    multi_engine_test_api::TestApi as RootTestApi, ApplyMigrations, CreateMigration, SchemaAssertion, SchemaPush,
};
use quaint::prelude::{Queryable, ResultSet};
use sql_migration_connector::SqlMigrationConnector;
use std::future::Future;
use test_setup::TestApiArgs;

pub struct TestApi {
    root: RootTestApi,
    connector: SqlMigrationConnector,
}

impl TestApi {
    /// Initializer, called by the test macros.
    pub fn new(args: TestApiArgs) -> Self {
        let root = RootTestApi::new(args);
        let connector = root.new_engine().connector;

        TestApi { root, connector }
    }

    /// Plan an `applyMigrations` command
    pub fn apply_migrations<'a>(&'a self, migrations_directory: &'a TempDir) -> ApplyMigrations<'a> {
        ApplyMigrations::new_sync(&self.connector, migrations_directory, &self.root.rt)
    }

    pub fn connection_string(&self) -> &str {
        self.root.connection_string()
    }

    /// Plan a `createMigration` command
    pub fn create_migration<'a>(
        &'a self,
        name: &'a str,
        schema: &'a str,
        migrations_directory: &'a TempDir,
    ) -> CreateMigration<'a> {
        CreateMigration::new_sync(&self.connector, name, schema, migrations_directory, &self.root.rt)
    }

    /// Create a temporary directory to serve as a test migrations directory.
    pub fn create_migrations_directory(&self) -> TempDir {
        self.root.create_migrations_directory()
    }

    /// Returns true only when testing on MSSQL.
    pub fn is_mssql(&self) -> bool {
        self.root.is_mssql()
    }

    /// Returns true only when testing on MySQL.
    pub fn is_mysql(&self) -> bool {
        self.root.is_mysql()
    }

    /// Returns true only when testing on MariaDB.
    pub fn is_mysql_mariadb(&self) -> bool {
        self.root.is_mysql_mariadb()
    }

    /// Returns true only when testing on MySQL 5.6.
    pub fn is_mysql_5_6(&self) -> bool {
        self.root.is_mysql_5_6()
    }

    /// Returns true only when testing on MySQL 8.
    pub fn is_mysql_8(&self) -> bool {
        self.root.is_mysql_8()
    }

    /// Returns true only when testing on postgres.
    pub fn is_postgres(&self) -> bool {
        self.root.is_postgres()
    }

    /// Returns true only when testing on sqlite.
    pub fn is_sqlite(&self) -> bool {
        self.root.is_sqlite()
    }

    /// Returns true only when testing on vitess.
    pub fn is_vitess(&self) -> bool {
        self.root.is_vitess()
    }

    /// Insert test values
    pub fn insert<'a>(&'a self, table_name: &'a str) -> SingleRowInsert<'a> {
        SingleRowInsert {
            insert: quaint::ast::Insert::single_into(self.render_table_name(table_name)),
            api: self,
        }
    }

    pub fn lower_cases_table_names(&self) -> bool {
        self.root.lower_cases_table_names()
    }

    pub fn migration_persistence<'a>(&'a self) -> &(dyn MigrationPersistence + 'a) {
        &self.connector
    }

    /// Assert facts about the database schema
    pub fn assert_schema(&self) -> SchemaAssertion {
        SchemaAssertion::new(
            self.root.block_on(self.connector.describe_schema()).unwrap(),
            self.root.args.tags(),
        )
    }

    /// Block on a future
    pub fn block_on<O, F: Future<Output = O>>(&self, f: F) -> O {
        self.root.block_on(f)
    }

    /// Render a valid datasource block, including database URL.
    pub fn datasource_block(&self) -> String {
        self.root.args.datasource_block(self.root.args.database_url())
    }

    /// Same as quaint::Queryable::query()
    pub fn query(&self, q: quaint::ast::Query<'_>) -> ResultSet {
        self.root.block_on(self.connector.quaint().query(q)).unwrap()
    }

    /// Send a SQL command to the database, and expect it to succeed.
    pub fn raw_cmd(&self, sql: &str) {
        self.root.raw_cmd(sql)
    }

    /// Render a table name with the required prefixing for use with quaint query building.
    pub fn render_table_name<'a>(&'a self, table_name: &'a str) -> quaint::ast::Table<'a> {
        if self.root.is_sqlite() {
            table_name.into()
        } else {
            (self.connector.quaint().connection_info().schema_name(), table_name).into()
        }
    }

    /// Plan a `schemaPush` command
    pub fn schema_push(&self, dm: impl Into<String>) -> SchemaPush<'_> {
        SchemaPush::new_sync(&self.connector, dm.into(), &self.root.rt)
    }

    pub fn tags(&self) -> BitFlags<Tags> {
        self.root.args.tags()
    }
}

pub struct SingleRowInsert<'a> {
    insert: quaint::ast::SingleRowInsert<'a>,
    api: &'a TestApi,
}

impl<'a> SingleRowInsert<'a> {
    /// Add a value to the row
    pub fn value(mut self, name: &'a str, value: impl Into<quaint::ast::Expression<'a>>) -> Self {
        self.insert = self.insert.value(name, value);

        self
    }

    /// Execute the request and return the result set.
    pub fn result_raw(self) -> quaint::connector::ResultSet {
        self.api.query(self.insert.into())
    }
}
