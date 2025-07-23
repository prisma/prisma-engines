pub use crate::assertions::{MigrationsAssertions, ResultSetExt, SchemaAssertion};
pub use expect_test::expect;
pub use schema_core::{
    json_rpc::types::{
        DbExecuteDatasourceType, DbExecuteParams, DiffParams, DiffResult, SchemaContainer, UrlContainer,
    },
    schema_connector::Namespaces,
};
pub use test_macros::test_connector;
pub use test_setup::{BitFlags, Capabilities, Tags, runtime::run_with_thread_local_runtime as tok};

use crate::{commands::*, multi_engine_test_api::TestApi as RootTestApi};
use psl::{
    datamodel_connector::NativeTypeInstance,
    parser_database::{ScalarType, SourceFile},
};
use quaint::{
    Value,
    prelude::{ConnectionInfo, ResultSet},
};
use schema_core::{
    commands::diff_cli,
    json_rpc::types::SchemaFilter,
    schema_connector::{BoxFuture, ConnectorHost, ConnectorResult, DiffTarget, MigrationPersistence, SchemaConnector},
};
use sql_schema_connector::SqlSchemaConnector;
use sql_schema_describer::SqlSchema;
use std::time::Duration;
use std::{
    borrow::Cow,
    fmt::{Display, Write},
};
use tempfile::TempDir;
use test_setup::{DatasourceBlock, TestApiArgs};

#[derive(Debug, Default)]
pub struct TestConnectorHost {
    pub printed_messages: std::sync::Mutex<Vec<String>>,
}

impl ConnectorHost for TestConnectorHost {
    fn print(&self, message: &str) -> BoxFuture<'_, ConnectorResult<()>> {
        // https://github.com/prisma/prisma/issues/11761
        assert!(message.ends_with('\n'));
        self.printed_messages.lock().unwrap().push(message.to_owned());
        Box::pin(std::future::ready(Ok(())))
    }
}

pub struct TestApi {
    root: RootTestApi,
    pub connector: SqlSchemaConnector,
}

impl TestApi {
    pub fn from_connector(connector: SqlSchemaConnector, args: TestApiArgs) -> Self {
        let root = RootTestApi::new(args);
        TestApi { root, connector }
    }

    /// Initializer, called by the test macros.
    pub fn new(args: TestApiArgs) -> Self {
        let root = RootTestApi::new(args);
        let connector = root.new_engine().connector;

        TestApi { root, connector }
    }

    pub fn args(&self) -> &TestApiArgs {
        &self.root.args
    }

    /// Plan an `applyMigrations` command
    pub fn apply_migrations<'a>(&'a mut self, migrations_directory: &'a TempDir) -> ApplyMigrations<'a> {
        let search_path = self.root.admin_conn.connection_info().schema_name().unwrap();
        let mut namespaces = vec![search_path.to_string()];

        for namespace in self.root.args.namespaces() {
            namespaces.push(namespace.to_string());
        }

        ApplyMigrations::new(&mut self.connector, migrations_directory, namespaces)
    }

    pub fn connection_string(&self) -> &str {
        self.root.connection_string()
    }

    pub fn connection_info(&self) -> ConnectionInfo {
        self.root.connection_info()
    }

    pub fn ensure_connection_validity(&mut self) -> ConnectorResult<()> {
        tok(self.connector.ensure_connection_validity())
    }

    pub fn schema_name(&self) -> String {
        self.connection_info().schema_name().unwrap().to_owned()
    }

    /// Plan a `createMigration` command.
    pub fn create_migration<'a>(
        &'a mut self,
        name: &'a str,
        schema: &'a str,
        migrations_directory: &'a TempDir,
    ) -> CreateMigration<'a> {
        CreateMigration::new(
            &mut self.connector,
            name,
            &[("schema.prisma", schema)],
            migrations_directory,
            SchemaFilter::default(),
            "",
        )
    }

    pub fn create_migration_with_filter<'a>(
        &'a mut self,
        name: &'a str,
        schema: &'a str,
        migrations_directory: &'a TempDir,
        filter: SchemaFilter,
        init_script: &'a str,
    ) -> CreateMigration<'a> {
        CreateMigration::new(
            &mut self.connector,
            name,
            &[("schema.prisma", schema)],
            migrations_directory,
            filter,
            init_script,
        )
    }

    pub fn create_migration_multi_file<'a>(
        &'a mut self,
        name: &'a str,
        files: &[(&'a str, &'a str)],
        migrations_directory: &'a TempDir,
    ) -> CreateMigration<'a> {
        CreateMigration::new(
            &mut self.connector,
            name,
            files,
            migrations_directory,
            SchemaFilter::default(),
            "",
        )
    }

    /// Create a temporary directory to serve as a test migrations directory.
    pub fn create_migrations_directory(&self) -> TempDir {
        self.root.create_migrations_directory()
    }

    /// Builder and assertions to call the `devDiagnostic` command.
    pub fn dev_diagnostic<'a>(&'a mut self, migrations_directory: &'a TempDir) -> DevDiagnostic<'a> {
        DevDiagnostic::new(&mut self.connector, migrations_directory, SchemaFilter::default())
    }

    pub fn dev_diagnostic_with_filter<'a>(
        &'a mut self,
        migrations_directory: &'a TempDir,
        filter: SchemaFilter,
    ) -> DevDiagnostic<'a> {
        DevDiagnostic::new(&mut self.connector, migrations_directory, filter)
    }

    pub fn diagnose_migration_history<'a>(
        &'a mut self,
        migrations_directory: &'a TempDir,
    ) -> DiagnoseMigrationHistory<'a> {
        DiagnoseMigrationHistory::new(&mut self.connector, migrations_directory)
    }

    pub fn diff(&self, params: DiffParams) -> ConnectorResult<DiffResult> {
        test_setup::runtime::run_with_thread_local_runtime(diff_cli(params, self.connector.host().clone()))
    }

    pub fn dump_table(&mut self, table_name: &str) -> ResultSet {
        let select_star =
            quaint::ast::Select::from_table(self.render_table_name(table_name)).value(quaint::ast::asterisk());

        self.query(select_star.into())
    }

    pub fn evaluate_data_loss<'a>(
        &'a mut self,
        migrations_directory: &'a TempDir,
        schema: String,
    ) -> EvaluateDataLoss<'a> {
        EvaluateDataLoss::new(
            &mut self.connector,
            migrations_directory,
            &[("schema.prisma", &schema)],
            SchemaFilter::default(),
        )
    }

    pub fn evaluate_data_loss_with_filter<'a>(
        &'a mut self,
        migrations_directory: &'a TempDir,
        schema: String,
        filter: SchemaFilter,
    ) -> EvaluateDataLoss<'a> {
        EvaluateDataLoss::new(
            &mut self.connector,
            migrations_directory,
            &[("schema.prisma", &schema)],
            filter,
        )
    }

    pub fn evaluate_data_loss_multi_file<'a>(
        &'a mut self,
        migrations_directory: &'a TempDir,
        files: &[(&'a str, &'a str)],
    ) -> EvaluateDataLoss<'a> {
        EvaluateDataLoss::new(
            &mut self.connector,
            migrations_directory,
            files,
            SchemaFilter::default(),
        )
    }

    pub fn introspect_sql<'a>(&'a mut self, name: &'a str, source: &'a str) -> IntrospectSql<'a> {
        let sanitized = self.sanitize_sql(source);

        IntrospectSql::new(&mut self.connector, name, sanitized)
    }

    // Replaces `?` with the appropriate positional parameter syntax for the current database.
    pub fn sanitize_sql(&self, sql: &str) -> String {
        let mut counter = 1;

        if self.is_mysql() || self.is_mariadb() || self.is_sqlite() {
            return sql.to_string();
        }

        let mut out = String::with_capacity(sql.len());
        let mut lines = sql.lines().peekable();

        while let Some(line) = lines.next() {
            // Avoid replacing query params in comments
            if line.trim_start().starts_with("--") {
                out.push_str(line);

                if lines.peek().is_some() {
                    out.push('\n');
                }
            } else {
                let mut line = line.to_string();

                while let Some(idx) = line.find('?') {
                    let replacer = if self.is_postgres() || self.is_cockroach() {
                        format!("${counter}")
                    } else if self.is_mssql() {
                        format!("@P{counter}")
                    } else {
                        unimplemented!()
                    };

                    line.replace_range(idx..idx + 1, &replacer);

                    counter += 1;
                }

                out.push_str(&line);

                if lines.peek().is_some() {
                    out.push('\n');
                }
            }
        }

        out
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

    /// Returns true only when testing on postgres version 15.
    pub fn is_postgres_15(&self) -> bool {
        self.root.is_postgres_15()
    }

    /// Returns true only when testing on postgres version 16.
    pub fn is_postgres_16(&self) -> bool {
        self.root.is_postgres_16()
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

    /// Returns a duration that is guaranteed to be larger than the maximum refresh rate after a
    /// DDL statement
    pub fn max_ddl_refresh_delay(&self) -> Option<Duration> {
        self.root.max_ddl_refresh_delay()
    }

    /// Insert test values
    pub fn insert<'a>(&'a mut self, table_name: &'a str) -> SingleRowInsert<'a> {
        SingleRowInsert {
            insert: quaint::ast::Insert::single_into(self.render_table_name(table_name)),
            api: self,
        }
    }

    pub fn lower_cases_table_names(&self) -> bool {
        self.root.lower_cases_table_names()
    }

    pub fn mark_migration_applied<'a>(
        &'a mut self,
        migration_name: impl Into<String>,
        migrations_directory: &'a TempDir,
    ) -> MarkMigrationApplied<'a> {
        MarkMigrationApplied::new(&mut self.connector, migration_name.into(), migrations_directory)
    }

    pub fn mark_migration_rolled_back(&mut self, migration_name: impl Into<String>) -> MarkMigrationRolledBack<'_> {
        MarkMigrationRolledBack::new(&mut self.connector, migration_name.into())
    }

    pub fn migration_persistence<'a>(&'a mut self) -> &'a mut (dyn MigrationPersistence + 'a) {
        &mut self.connector
    }

    /// Assert facts about the database schema
    #[track_caller]
    pub fn assert_schema(&mut self) -> SchemaAssertion {
        let schema: SqlSchema = tok(self.connector.describe_schema(None)).unwrap();
        SchemaAssertion::new(schema, self.root.args.tags())
    }

    #[track_caller]
    pub fn assert_schema_with_namespaces(&mut self, namespaces: Option<Namespaces>) -> SchemaAssertion {
        let schema: SqlSchema = tok(self.connector.describe_schema(namespaces)).unwrap();
        SchemaAssertion::new(schema, self.root.args.tags())
    }

    /// Render a valid datasource block, including database URL.
    pub fn datasource_block(&self) -> DatasourceBlock<'_> {
        self.root.datasource_block()
    }

    pub fn datasource_block_with<'a>(&'a self, params: &'a [(&'a str, &'a str)]) -> DatasourceBlock<'a> {
        self.root
            .args
            .datasource_block(self.root.connection_string(), params, &[])
    }

    /// Generate a migration script using `MigrationConnector::diff()`.
    pub fn connector_diff(
        &mut self,
        from: DiffTarget<'_>,
        to: DiffTarget<'_>,
        namespaces: Option<Namespaces>,
    ) -> String {
        let default_namespace = self.connector.default_namespace().map(|s| s.to_string());

        let from = tok(self.connector.schema_from_diff_target(
            from,
            namespaces.clone(),
            default_namespace.as_deref(),
            &SchemaFilter::default().into(),
        ))
        .unwrap();
        let to = tok(self.connector.schema_from_diff_target(
            to,
            namespaces,
            default_namespace.as_deref(),
            &SchemaFilter::default().into(),
        ))
        .unwrap();
        let dialect = self.connector.schema_dialect();
        let migration = dialect.diff(from, to, &SchemaFilter::default().into());
        dialect.render_script(&migration, &Default::default()).unwrap()
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
    pub fn query(&mut self, q: quaint::ast::Query<'_>) -> ResultSet {
        tok(self.connector.query(q)).unwrap()
    }

    /// Like quaint::Queryable::query_raw()
    #[track_caller]
    pub fn query_raw(&mut self, q: &str, params: &[Value<'static>]) -> ResultSet {
        tok(self.connector.query_raw(q, params)).unwrap()
    }

    /// Send a SQL command to the database, and expect it to succeed.
    #[track_caller]
    pub fn raw_cmd(&mut self, sql: &str) {
        tok(self.connector.raw_cmd(sql)).unwrap()
    }

    /// Render a table name with the required prefixing for use with quaint query building.
    pub fn render_table_name(&self, table_name: &str) -> quaint::ast::Table<'static> {
        let Some(schema_name) = self.connection_info().schema_name().map(<_>::to_owned) else {
            return table_name.to_owned().into();
        };

        if self.root.is_sqlite() {
            table_name.to_owned().into()
        } else {
            (schema_name, table_name.to_owned()).into()
        }
    }

    /// Plan a `reset` command
    pub fn reset(&mut self) -> Reset<'_> {
        Reset::new(&mut self.connector)
    }

    pub fn expect_sql_for_schema(&mut self, schema: &'static str, sql: &expect_test::Expect) {
        let found = self.connector_diff(
            DiffTarget::Empty,
            DiffTarget::Datamodel(vec![("schema.prisma".to_string(), SourceFile::new_static(schema))]),
            None,
        );
        sql.assert_eq(&found);
    }

    /// Plan a `schemaPush` command adding the datasource
    pub fn schema_push_w_datasource(&mut self, dm: impl Into<String>) -> SchemaPush<'_> {
        let schema = self.datamodel_with_provider(&dm.into());
        self.schema_push(schema)
    }

    pub fn schema_push_w_datasource_multi_file(&mut self, files: &[(&str, &str)]) -> SchemaPush<'_> {
        let (first, rest) = files.split_first().unwrap();
        let first_with_provider = self.datamodel_with_provider(first.1);
        let recombined = [&[(first.0, first_with_provider.as_str())], rest].concat();

        self.schema_push_multi_file(&recombined)
    }

    /// Plan a `schemaPush` command
    pub fn schema_push(&mut self, dm: impl Into<String>) -> SchemaPush<'_> {
        self.schema_push_with_filter(dm, None)
    }

    pub fn schema_push_multi_file(&mut self, files: &[(&str, &str)]) -> SchemaPush<'_> {
        let max_ddl_refresh_delay = self.max_ddl_refresh_delay();
        SchemaPush::new(&mut self.connector, files, max_ddl_refresh_delay, None)
    }

    pub fn schema_push_with_filter(&mut self, dm: impl Into<String>, filter: Option<SchemaFilter>) -> SchemaPush<'_> {
        let max_ddl_refresh_delay = self.max_ddl_refresh_delay();
        let dm: String = dm.into();

        SchemaPush::new(
            &mut self.connector,
            &[("schema.prisma", &dm)],
            max_ddl_refresh_delay,
            filter,
        )
    }

    pub fn tags(&self) -> BitFlags<Tags> {
        self.root.args.tags()
    }

    /// Render a valid datasource block, including database URL.
    pub fn write_datasource_block(
        &self,
        out: &mut dyn std::fmt::Write,
        params: &[(&str, &str)],
        preview_features: &'static [&'static str],
    ) {
        let no_foreign_keys = self.is_vitess();

        let used_params = if no_foreign_keys && params.is_empty() {
            vec![("relationMode", r#""prisma""#)]
        } else {
            params.to_vec()
        };

        let ds_block = self
            .root
            .args
            .datasource_block(self.root.args.database_url(), &used_params, preview_features);

        write!(out, "{ds_block}").unwrap()
    }

    pub fn generator_block(&self) -> String {
        let preview_feature_string = if self.root.preview_features().is_empty() {
            "".to_string()
        } else {
            let features = self
                .root
                .preview_features()
                .iter()
                .map(|f| format!(r#""{f}""#))
                .join(", ");

            format!("\npreviewFeatures = [{features}]")
        };

        let generator_block = format!(
            r#"generator client {{
                 provider = "prisma-client-js"{preview_feature_string}
               }}"#
        );
        generator_block
    }

    pub fn datamodel_with_provider(&self, schema: &str) -> String {
        let mut out = String::with_capacity(320 + schema.len());

        self.write_datasource_block(&mut out, &[], &[]);
        out.push('\n');
        out.push_str(&self.generator_block());
        out.push_str(schema);

        out
    }

    pub fn datamodel_with_provider_and_features(
        &self,
        schema: &str,
        params: &[(&str, &str)],
        preview_features: &'static [&'static str],
    ) -> String {
        let mut out = String::with_capacity(320 + schema.len());

        self.write_datasource_block(&mut out, params, preview_features);
        out.push('\n');
        out.push_str(&self.generator_block());
        out.push_str(schema);

        out
    }

    pub fn scalar_type_for_native_type(&self, typ: &NativeTypeInstance) -> ScalarType {
        self.connector.scalar_type_for_native_type(typ)
    }
}

pub struct SingleRowInsert<'a> {
    insert: quaint::ast::SingleRowInsert<'a>,
    api: &'a mut TestApi,
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
            write!(out, "{first_item}").unwrap();
        }

        for item in self {
            out.push_str(sep);
            write!(out, "{item}").unwrap();
        }

        out
    }
}
