pub use crate::assertions::{MigrationsAssertions, ResultSetExt, SchemaAssertion};
use datamodel::common::preview_features::PreviewFeature;
pub use expect_test::expect;
pub use test_macros::test_connector;
pub use test_setup::{BitFlags, Capabilities, Tags};

use crate::{commands::*, multi_engine_test_api::TestApi as RootTestApi};
use migration_connector::{
    ConnectorResult, DatabaseMigrationStepApplier, DiffTarget, MigrationConnector, MigrationPersistence,
};
use quaint::{
    prelude::{ConnectionInfo, ResultSet},
    Value,
};
use sql_migration_connector::SqlMigrationConnector;
use std::{
    borrow::Cow,
    fmt::{Display, Write},
    future::Future,
};
use tempfile::TempDir;
use test_setup::{DatasourceBlock, TestApiArgs};

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

    pub fn connection_info(&self) -> ConnectionInfo {
        self.root.connection_info()
    }

    pub fn ensure_connection_validity(&self) -> ConnectorResult<()> {
        self.block_on(self.connector.ensure_connection_validity())
    }

    pub fn schema_name(&self) -> String {
        self.connection_info().schema_name().to_owned()
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

    /// Builder and assertions to call the `devDiagnostic` command.
    pub fn dev_diagnostic<'a>(&'a self, migrations_directory: &'a TempDir) -> DevDiagnostic<'a> {
        DevDiagnostic::new(&self.connector, migrations_directory, &self.root.rt)
    }

    pub fn diagnose_migration_history<'a>(&'a self, migrations_directory: &'a TempDir) -> DiagnoseMigrationHistory<'a> {
        DiagnoseMigrationHistory::new_sync(&self.connector, migrations_directory, &self.root.rt)
    }

    pub fn dump_table(&self, table_name: &str) -> ResultSet {
        let select_star =
            quaint::ast::Select::from_table(self.render_table_name(table_name)).value(quaint::ast::asterisk());

        self.query(select_star.into())
    }

    pub fn evaluate_data_loss<'a>(&'a self, migrations_directory: &'a TempDir, schema: String) -> EvaluateDataLoss<'a> {
        EvaluateDataLoss::new(&self.connector, migrations_directory, schema, &self.root.rt)
    }

    /// Returns true only when testing on MSSQL.
    pub fn is_mssql(&self) -> bool {
        self.root.is_mssql()
    }

    /// Returns true only when testing on MariaDB.
    pub fn is_mariadb(&self) -> bool {
        self.root.is_mysql_mariadb()
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

    /// Returns true only when testing on cockroach.
    pub fn is_cockroach(&self) -> bool {
        self.root.is_cockroach()
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

    pub fn list_migration_directories<'a>(&'a self, migrations_directory: &'a TempDir) -> ListMigrationDirectories<'a> {
        ListMigrationDirectories::new(&self.connector, migrations_directory, &self.root.rt)
    }

    pub fn lower_cases_table_names(&self) -> bool {
        self.root.lower_cases_table_names()
    }

    pub fn mark_migration_applied<'a>(
        &'a self,
        migration_name: impl Into<String>,
        migrations_directory: &'a TempDir,
    ) -> MarkMigrationApplied<'a> {
        MarkMigrationApplied::new(
            &self.connector,
            migration_name.into(),
            migrations_directory,
            &self.root.rt,
        )
    }

    pub fn mark_migration_rolled_back(&self, migration_name: impl Into<String>) -> MarkMigrationRolledBack<'_> {
        MarkMigrationRolledBack::new(&self.connector, migration_name.into(), &self.root.rt)
    }

    pub fn migration_persistence<'a>(&'a self) -> &(dyn MigrationPersistence + 'a) {
        &self.connector
    }

    /// Assert facts about the database schema
    #[track_caller]
    pub fn assert_schema(&self) -> SchemaAssertion {
        SchemaAssertion::new(
            self.root.block_on(self.connector.describe_schema()).unwrap(),
            self.root.args.tags(),
        )
    }

    /// Block on a future.
    pub fn block_on<O, F: Future<Output = O>>(&self, f: F) -> O {
        self.root.block_on(f)
    }

    /// Render a valid datasource block, including database URL.
    pub fn datasource_block(&self) -> DatasourceBlock<'_> {
        self.root.datasource_block()
    }

    pub fn datasource_block_with<'a>(&'a self, params: &'a [(&'a str, &'a str)]) -> DatasourceBlock<'a> {
        self.root.args.datasource_block(self.root.connection_string(), params)
    }

    /// Generate a migration script using `MigrationConnector::diff()`.
    pub fn diff(&self, from: DiffTarget<'_>, to: DiffTarget<'_>) -> String {
        let migration = self.block_on(self.connector.diff(from, to)).unwrap();
        self.connector.render_script(&migration, &Default::default())
    }

    pub fn normalize_identifier<'a>(&self, identifier: &'a str) -> Cow<'a, str> {
        if self.lower_cases_table_names() {
            identifier.to_ascii_lowercase().into()
        } else {
            identifier.into()
        }
    }

    /// Like quaint::Queryable::query()
    #[track_caller]
    pub fn query(&self, q: quaint::ast::Query<'_>) -> ResultSet {
        self.root.block_on(self.connector.query(q)).unwrap()
    }

    /// Like quaint::Queryable::query_raw()
    #[track_caller]
    pub fn query_raw(&self, q: &str, params: &[Value<'static>]) -> ResultSet {
        self.root.block_on(self.connector.query_raw(q, params)).unwrap()
    }

    /// Send a SQL command to the database, and expect it to succeed.
    #[track_caller]
    pub fn raw_cmd(&self, sql: &str) {
        self.root.block_on(self.connector.raw_cmd(sql)).unwrap()
    }

    /// Render a table name with the required prefixing for use with quaint query building.
    pub fn render_table_name<'a>(&'a self, table_name: &'a str) -> quaint::ast::Table<'a> {
        if self.root.is_sqlite() {
            table_name.into()
        } else {
            (self.connection_info().schema_name().to_owned(), table_name.to_owned()).into()
        }
    }

    /// Plan a `reset` command
    pub fn reset(&self) -> Reset<'_> {
        Reset::new_sync(&self.connector, &self.root.rt)
    }

    /// Plan a `schemaPush` command adding the datasource
    pub fn schema_push_w_datasource(&self, dm: impl Into<String>) -> SchemaPush<'_> {
        SchemaPush::new(&self.connector, self.datamodel_with_provider(&dm.into()), &self.root.rt)
    }

    /// Plan a `schemaPush` command
    pub fn schema_push(&self, dm: impl Into<String>) -> SchemaPush<'_> {
        SchemaPush::new(&self.connector, dm.into(), &self.root.rt)
    }

    pub fn tags(&self) -> BitFlags<Tags> {
        self.root.args.tags()
    }

    /// Render a valid datasource block, including database URL.
    pub fn write_datasource_block(&self, out: &mut dyn std::fmt::Write) {
        let no_foreign_keys = self.is_vitess()
            && self
                .root
                .preview_features()
                .contains(PreviewFeature::ReferentialIntegrity);

        let params = if no_foreign_keys {
            vec![("referentialIntegrity", r#""prisma""#)]
        } else {
            Vec::new()
        };

        write!(
            out,
            "{}",
            self.root.args.datasource_block(self.root.args.database_url(), &params)
        )
        .unwrap()
    }

    fn generator_block(&self) -> String {
        let preview_feature_string = if self.root.preview_features().is_empty() {
            "".to_string()
        } else {
            let features = self
                .root
                .preview_features()
                .iter()
                .map(|f| format!(r#""{}""#, f))
                .join(", ");

            format!("\npreviewFeatures = [{}]", features)
        };

        let generator_block = format!(
            r#"generator client {{
                 provider = "prisma-client-js"{}
               }}"#,
            preview_feature_string
        );
        generator_block
    }

    pub fn datamodel_with_provider(&self, schema: &str) -> String {
        let mut out = String::with_capacity(320 + schema.len());

        self.write_datasource_block(&mut out);
        out.push('\n');
        out.push_str(&self.generator_block());
        out.push_str(schema);

        out
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

pub(crate) trait IteratorJoin {
    fn join(self, sep: &str) -> String;
}

impl<T, I> IteratorJoin for T
where
    T: Iterator<Item = I>,
    I: Display,
{
    fn join(mut self, sep: &str) -> String {
        let (lower_bound, _) = self.size_hint();
        let mut out = String::with_capacity(sep.len() * lower_bound);

        if let Some(first_item) = self.next() {
            write!(out, "{}", first_item).unwrap();
        }

        for item in self {
            out.push_str(sep);
            write!(out, "{}", item).unwrap();
        }

        out
    }
}
