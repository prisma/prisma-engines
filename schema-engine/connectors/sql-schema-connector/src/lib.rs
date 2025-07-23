//! The SQL migration connector.

#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

mod apply_migration;
mod database_schema;
mod error;
mod flavour;
mod introspection;
mod migration_pair;
mod sql_destructive_change_checker;
mod sql_doc_parser;
mod sql_migration;
mod sql_migration_persistence;
mod sql_renderer;
mod sql_schema_calculator;
mod sql_schema_differ;

use database_schema::SqlDatabaseSchema;
use enumflags2::BitFlags;
use flavour::{SqlConnector, SqlDialect, UsingExternalShadowDb};
use migration_pair::MigrationPair;
use psl::{SourceFile, ValidatedSchema, datamodel_connector::NativeTypeInstance, parser_database::ScalarType};
use quaint::connector::DescribedQuery;
use schema_connector::{migrations_directory::Migrations, *};
use sql_doc_parser::{parse_sql_doc, sanitize_sql};
use sql_migration::{DropUserDefinedType, DropView, SqlMigration, SqlMigrationStep};
use sql_schema_describer as sql;
use std::{future, sync::Arc};

const MIGRATIONS_TABLE_NAME: &str = "_prisma_migrations";

/// A SQL schema dialect.
pub struct SqlSchemaDialect {
    dialect: Box<dyn SqlDialect>,
}

impl SqlSchemaDialect {
    /// Creates a CockroachDB schema dialect with the default settings.
    #[cfg(feature = "postgresql")]
    pub fn cockroach() -> Self {
        Self::new(Box::new(flavour::PostgresDialect::cockroach()))
    }

    /// Creates a PostgreSQL schema dialect with the default settings.
    #[cfg(feature = "postgresql")]
    pub fn postgres() -> Self {
        Self::new(Box::new(flavour::PostgresDialect::default()))
    }

    /// Creates a MySQL schema dialect with the default settings.
    #[cfg(feature = "mysql")]
    pub fn mysql() -> Self {
        Self::new(Box::new(flavour::MysqlDialect::default()))
    }

    /// Creates a SQLite schema dialect with the default settings.
    #[cfg(feature = "sqlite")]
    pub fn sqlite() -> Self {
        Self::new(Box::new(flavour::SqliteDialect))
    }

    /// Creates a SQL Server schema dialect with the default settings.
    #[cfg(feature = "mssql")]
    pub fn mssql() -> Self {
        Self::new(Box::new(flavour::MssqlDialect::default()))
    }

    fn new(flavour: Box<dyn SqlDialect>) -> Self {
        Self { dialect: flavour }
    }
}

impl SchemaDialect for SqlSchemaDialect {
    #[tracing::instrument(skip(self, from, to))]
    fn diff(&self, from: DatabaseSchema, to: DatabaseSchema, filter: &SchemaFilter) -> Migration {
        let previous = SqlDatabaseSchema::from_erased(from);
        let next = SqlDatabaseSchema::from_erased(to);
        let steps = sql_schema_differ::calculate_steps(
            MigrationPair::new(&previous, &next),
            &*self.dialect.schema_differ(),
            filter,
        );
        tracing::debug!(?steps, "Inferred migration steps.");

        Migration::new(SqlMigration {
            before: previous.describer_schema,
            after: next.describer_schema,
            steps,
        })
    }

    fn empty_database_schema(&self) -> DatabaseSchema {
        DatabaseSchema::new(SqlDatabaseSchema::from(self.dialect.empty_database_schema()))
    }

    fn migration_file_extension(&self) -> &'static str {
        "sql"
    }

    fn migration_len(&self, migration: &Migration) -> usize {
        migration.downcast_ref::<SqlMigration>().steps.len()
    }

    fn render_script(
        &self,
        migration: &Migration,
        diagnostics: &DestructiveChangeDiagnostics,
    ) -> ConnectorResult<String> {
        apply_migration::render_script(migration, diagnostics, &*self.dialect.renderer())
    }

    fn migration_summary(&self, migration: &Migration) -> String {
        migration.downcast_ref::<SqlMigration>().drift_summary()
    }

    fn extract_namespaces(&self, schema: &DatabaseSchema) -> Option<Namespaces> {
        let sql_schema: &SqlDatabaseSchema = schema.downcast_ref();
        Namespaces::from_vec(
            &mut sql_schema
                .describer_schema
                .walk_namespaces()
                .map(|nw| String::from(nw.name()))
                .collect::<Vec<String>>(),
        )
    }

    fn schema_from_datamodel(
        &self,
        sources: Vec<(String, SourceFile)>,
        default_namespace: Option<&str>,
    ) -> ConnectorResult<DatabaseSchema> {
        let default_namespace = match default_namespace {
            Some(ns) => ns,
            None => unreachable!("Default namespace is required for SQL schema connector"),
        };

        let schema = psl::parse_schema_multi(&sources).map_err(ConnectorError::new_schema_parser_error)?;
        self.dialect.check_schema_features(&schema)?;
        let calculator = self.dialect.schema_calculator();
        Ok(sql_schema_calculator::calculate_sql_schema(&schema, default_namespace, &*calculator).into())
    }

    #[tracing::instrument(skip(self, migrations, target))]
    fn validate_migrations_with_target<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
        target: ExternalShadowDatabase,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        Box::pin(async move {
            self.schema_from_migrations_with_target(migrations, namespaces, filter, target)
                .await?;
            Ok(())
        })
    }

    fn schema_from_migrations_with_target<'a>(
        &'a self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
        target: ExternalShadowDatabase,
    ) -> BoxFuture<'a, ConnectorResult<DatabaseSchema>> {
        Box::pin(async move {
            let mut connector = match target {
                #[cfg(not(any(
                    feature = "mssql-native",
                    feature = "mysql-native",
                    feature = "postgresql-native",
                    feature = "sqlite-native"
                )))]
                ExternalShadowDatabase::DriverAdapter(factory) => self.dialect.connect_to_shadow_db(factory).await?,
                #[cfg(any(
                    feature = "mssql-native",
                    feature = "mysql-native",
                    feature = "postgresql-native",
                    feature = "sqlite-native"
                ))]
                ExternalShadowDatabase::ConnectionString {
                    connection_string,
                    preview_features,
                } => {
                    self.dialect
                        .connect_to_shadow_db(connection_string, preview_features)
                        .await?
                }
                _ => {
                    return Err(ConnectorError::from_msg(
                        "Received an unsupported shadow database target".to_owned(),
                    ));
                }
            };
            let schema = connector
                .sql_schema_from_migration_history(migrations, namespaces, filter, UsingExternalShadowDb::Yes)
                .await;
            // dispose of the connector regardless of the result
            connector.dispose().await?;
            Ok(DatabaseSchema::new(SqlDatabaseSchema::from(schema?)))
        })
    }
}

/// The top-level SQL migration connector.
pub struct SqlSchemaConnector {
    inner: Box<dyn SqlConnector + Send + Sync + 'static>,
    host: Arc<dyn ConnectorHost>,
}

impl SqlSchemaConnector {
    /// Initialise an external migration connector.
    pub async fn new_from_external(adapter: Arc<dyn quaint::connector::ExternalConnector>) -> ConnectorResult<Self> {
        match adapter.provider() {
            #[cfg(all(feature = "postgresql", not(feature = "postgresql-native")))]
            quaint::connector::AdapterProvider::Postgres => Self::new_postgres_external(adapter).await,
            #[cfg(all(feature = "sqlite", not(feature = "sqlite-native")))]
            quaint::connector::AdapterProvider::Sqlite => Ok(Self::new_sqlite_external(adapter).await),
            #[allow(unreachable_patterns)]
            _ => panic!("Unsupported adapter provider: {:?}", adapter.provider()),
        }
    }

    /// Initialize an external PostgreSQL migration connector.
    #[cfg(all(feature = "postgresql", not(feature = "postgresql-native")))]
    pub async fn new_postgres_external(
        adapter: Arc<dyn quaint::connector::ExternalConnector>,
    ) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            inner: Box::new(flavour::PostgresConnector::new_external(adapter).await?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize an external SQLite migration connector.
    #[cfg(all(feature = "sqlite", not(feature = "sqlite-native")))]
    pub async fn new_sqlite_external(adapter: Arc<dyn quaint::connector::ExternalConnector>) -> Self {
        SqlSchemaConnector {
            inner: Box::new(flavour::SqliteConnector::new_external(adapter)),
            host: Arc::new(EmptyHost),
        }
    }

    /// Initialize a PostgreSQL migration connector.
    #[cfg(feature = "postgresql-native")]
    pub fn new_postgres(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            inner: Box::new(flavour::PostgresConnector::new_postgres(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize a CockroachDb migration connector.
    #[cfg(feature = "cockroachdb-native")]
    pub fn new_cockroach(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            inner: Box::new(flavour::PostgresConnector::new_cockroach(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize a PostgreSQL-like schema connector.
    ///
    /// Use [`Self::new_postgres()`] or [`Self::new_cockroach()`] instead when the provider is
    /// explicitly specified by user or already known otherwise.
    #[cfg(any(feature = "postgresql-native", feature = "cockroachdb-native"))]
    pub fn new_postgres_like(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            inner: Box::new(flavour::PostgresConnector::new_with_params(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize a SQLite migration connector.
    #[cfg(feature = "sqlite-native")]
    pub fn new_sqlite(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            inner: Box::new(flavour::SqliteConnector::new_with_params(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize a SQLite migration connector.
    #[cfg(feature = "sqlite-native")]
    pub fn new_sqlite_inmem(preview_features: psl::PreviewFeatures) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            inner: Box::new(flavour::SqliteConnector::new_inmem(preview_features)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize a MySQL migration connector.
    #[cfg(feature = "mysql-native")]
    pub fn new_mysql(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            inner: Box::new(flavour::MysqlConnector::new_with_params(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize a MSSQL migration connector.
    #[cfg(feature = "mssql-native")]
    pub fn new_mssql(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            inner: Box::new(flavour::MssqlConnector::new_with_params(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Returns the SQL dialect used by the connector.
    fn sql_dialect(&self) -> Box<dyn SqlDialect> {
        self.inner.dialect()
    }

    /// Made public for tests.
    pub fn describe_schema(
        &mut self,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'_, ConnectorResult<sql::SqlSchema>> {
        self.inner.describe_schema(namespaces)
    }

    /// For tests
    pub async fn query_raw(
        &mut self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        self.inner.query_raw(sql, params).await
    }

    /// For tests
    pub async fn query(
        &mut self,
        query: impl Into<quaint::ast::Query<'_>>,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        self.inner.query(query.into()).await
    }

    /// For tests
    pub async fn raw_cmd(&mut self, sql: &str) -> ConnectorResult<()> {
        self.inner.raw_cmd(sql).await
    }

    /// Returns the native types that can be used to represent the given scalar type.
    pub fn scalar_type_for_native_type(&self, native_type: &NativeTypeInstance) -> ScalarType {
        self.inner
            .dialect()
            .datamodel_connector()
            .scalar_type_for_native_type(native_type)
    }
}

impl SchemaConnector for SqlSchemaConnector {
    fn schema_dialect(&self) -> Box<dyn SchemaDialect> {
        Box::new(SqlSchemaDialect::new(self.inner.dialect()))
    }

    fn default_namespace(&self) -> Option<&str> {
        Some(self.inner.search_path())
    }

    // TODO: this only seems to be used in `sql-migration-tests`.
    fn set_host(&mut self, host: Arc<dyn schema_connector::ConnectorHost>) {
        self.host = host;
    }

    fn set_preview_features(&mut self, preview_features: BitFlags<psl::PreviewFeature>) {
        self.inner.set_preview_features(preview_features)
    }

    fn connector_type(&self) -> &'static str {
        self.inner.connector_type()
    }

    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        // If the env is set and non empty or set to `0`, we disable the lock.
        // TODO: avoid using `std::env::var` in Wasm.
        let disable_lock: bool = std::env::var("PRISMA_SCHEMA_DISABLE_ADVISORY_LOCK")
            .ok()
            .map(|value| !matches!(value.as_str(), "0" | ""))
            .unwrap_or(false);

        if disable_lock {
            tracing::info!(
                "PRISMA_SCHEMA_DISABLE_ADVISORY_LOCK environnement variable is set. Advisory lock is disabled."
            );
            return Box::pin(future::ready(Ok(())));
        }
        Box::pin(self.inner.acquire_lock())
    }

    fn apply_migration<'a>(&'a mut self, migration: &'a Migration) -> BoxFuture<'a, ConnectorResult<u32>> {
        Box::pin(apply_migration::apply_migration(migration, self.inner.as_mut()))
    }

    fn apply_script<'a>(&'a mut self, migration_name: &'a str, script: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        Box::pin(apply_migration::apply_script(migration_name, script, self))
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.inner.ensure_connection_validity()
    }

    // TODO: this only seems to be used in `sql-migration-tests`.
    fn host(&self) -> &Arc<dyn ConnectorHost> {
        &self.host
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(async {
            self.inner
                .version()
                .await
                .map(|version| version.unwrap_or_else(|| "Database version information not available.".to_owned()))
        })
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        self.inner.create_database()
    }

    fn schema_from_database(
        &mut self,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'_, ConnectorResult<DatabaseSchema>> {
        Box::pin(async move {
            self.inner
                .describe_schema(namespaces)
                .await
                .map(SqlDatabaseSchema::from)
                .map(DatabaseSchema::new)
        })
    }

    fn schema_from_migrations<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
    ) -> BoxFuture<'a, ConnectorResult<DatabaseSchema>> {
        Box::pin(async move {
            match self.inner.shadow_db_url() {
                Some(connection_string) => {
                    let target = ExternalShadowDatabase::ConnectionString {
                        connection_string: connection_string.to_owned(),
                        preview_features: self.inner.preview_features(),
                    };
                    self.schema_dialect()
                        .schema_from_migrations_with_target(migrations, namespaces, filter, target)
                        .await
                }
                None => self
                    .inner
                    .sql_schema_from_migration_history(migrations, namespaces, filter, UsingExternalShadowDb::No)
                    .await
                    .map(SqlDatabaseSchema::from)
                    .map(DatabaseSchema::new),
            }
        })
    }

    fn db_execute(&mut self, script: String) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async move { self.inner.raw_cmd(&script).await })
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.inner.drop_database()
    }

    fn introspect<'a>(
        &'a mut self,
        ctx: &'a IntrospectionContext,
    ) -> BoxFuture<'a, ConnectorResult<IntrospectionResult>> {
        Box::pin(async move {
            let mut namespace_names = match ctx.namespaces() {
                Some(namespaces) => namespaces.iter().map(|s| s.to_string()).collect(),
                None => ctx.datasource().namespaces.iter().map(|(s, _)| s.to_string()).collect(),
            };

            let namespaces = Namespaces::from_vec(&mut namespace_names);
            let sql_schema = self.inner.introspect(namespaces, ctx).await?;
            let search_path = self.inner.search_path();

            let datamodel = introspection::datamodel_calculator::calculate(&sql_schema, ctx, search_path);

            Ok(datamodel)
        })
    }

    fn reset<'a>(
        &'a mut self,
        soft: bool,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        Box::pin(async move {
            if soft || self.inner.reset(namespaces.clone()).await.is_err() {
                best_effort_reset(self.inner.as_mut(), namespaces, filter).await?;
            }

            Ok(())
        })
    }

    /// Optionally check that the features implied by the provided datamodel are all compatible with
    /// the specific database version being used.
    fn check_database_version_compatibility(
        &self,
        datamodel: &ValidatedSchema,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        self.inner.check_database_version_compatibility(datamodel)
    }

    fn destructive_change_checker(&mut self) -> &mut dyn DestructiveChangeChecker {
        self
    }

    fn migration_persistence(&mut self) -> &mut dyn MigrationPersistence {
        self
    }

    #[tracing::instrument(skip(self, migrations))]
    fn validate_migrations<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        Box::pin(async move {
            self.schema_from_migrations(migrations, namespaces, filter).await?;
            Ok(())
        })
    }

    fn introspect_sql(
        &mut self,
        input: IntrospectSqlQueryInput,
    ) -> BoxFuture<'_, ConnectorResult<IntrospectSqlQueryOutput>> {
        Box::pin(async move {
            let sanitized_sql = sanitize_sql(&input.source);
            let DescribedQuery {
                parameters,
                columns,
                enum_names,
            } = self.inner.describe_query(&sanitized_sql).await?;
            let enum_names = enum_names.unwrap_or_default();
            let sql_source = input.source.clone();
            let parsed_doc = parse_sql_doc(&sql_source, enum_names.as_slice())?;

            let parameters = parameters
                .into_iter()
                .zip(1..)
                .map(|(param, idx)| {
                    let parsed_param = parsed_doc
                        .get_param_at(idx)
                        .or_else(|| parsed_doc.get_param_by_alias(&param.name));

                    IntrospectSqlQueryParameterOutput {
                        typ: parsed_param
                            .and_then(|p| p.typ())
                            .unwrap_or_else(|| param.typ.to_string()),
                        name: parsed_param
                            .and_then(|p| p.alias())
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| param.name),
                        documentation: parsed_param.and_then(|p| p.documentation()).map(ToOwned::to_owned),
                        // Params are required by default unless overridden by sql doc.
                        nullable: parsed_param.and_then(|p| p.nullable()).unwrap_or(false),
                    }
                })
                .collect();
            let columns = columns.into_iter().map(IntrospectSqlQueryColumnOutput::from).collect();

            Ok(IntrospectSqlQueryOutput {
                name: input.name,
                source: sanitized_sql,
                documentation: parsed_doc.description().map(ToOwned::to_owned),
                parameters,
                result_columns: columns,
            })
        })
    }

    fn dispose(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.inner.dispose()
    }
}

fn new_shadow_database_name() -> String {
    format!("prisma_migrate_shadow_db_{}", uuid::Uuid::new_v4())
}

/// Try to reset the database to an empty state. This should only be used
/// when we don't have the permissions to do a full reset.
#[tracing::instrument(skip(connector))]
async fn best_effort_reset(
    connector: &mut (dyn SqlConnector + Send + Sync),
    namespaces: Option<Namespaces>,
    filter: &SchemaFilter,
) -> ConnectorResult<()> {
    best_effort_reset_impl(connector, namespaces, filter)
        .await
        .map_err(|err| err.into_soft_reset_failed_error())
}

async fn best_effort_reset_impl(
    connector: &mut (dyn SqlConnector + Send + Sync),
    namespaces: Option<Namespaces>,
    filter: &SchemaFilter,
) -> ConnectorResult<()> {
    tracing::info!("Attempting best_effort_reset");

    let dialect = connector.dialect();
    let source_schema = connector.describe_schema(namespaces).await?;
    let target_schema = dialect.empty_database_schema();
    let mut steps = Vec::new();

    // We drop views here, not in the normal migration process to not
    // accidentally drop something we can't describe in the data model.
    let drop_views = source_schema
        .view_walkers()
        .filter(|view| !dialect.schema_differ().view_should_be_ignored(view.name()))
        .map(|vw| DropView::new(vw.id))
        .map(SqlMigrationStep::DropView);

    steps.extend(drop_views);

    let diffables: MigrationPair<SqlDatabaseSchema> = MigrationPair::new(source_schema, target_schema).map(From::from);
    steps.extend(sql_schema_differ::calculate_steps(
        diffables.as_ref(),
        &*dialect.schema_differ(),
        filter,
    ));
    let (source_schema, target_schema) = diffables.map(|s| s.describer_schema).into_tuple();

    let drop_udts = source_schema
        .udt_walkers()
        .map(|udtw| udtw.id)
        .map(DropUserDefinedType::new)
        .map(SqlMigrationStep::DropUserDefinedType);

    steps.extend(drop_udts);

    let migration = SqlMigration {
        before: source_schema,
        after: target_schema,
        steps,
    };

    if migration.before.table_walker(crate::MIGRATIONS_TABLE_NAME).is_some() {
        connector.drop_migrations_table().await?;
    }

    if migration.steps.is_empty() {
        return Ok(());
    }

    let migration = apply_migration::render_script(
        &Migration::new(migration),
        &DestructiveChangeDiagnostics::default(),
        &*dialect.renderer(),
    )?;

    connector.raw_cmd(&migration).await?;

    Ok(())
}
