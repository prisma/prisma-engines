//! The SQL migration connector.

#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

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
use flavour::SqlFlavour;
use migration_pair::MigrationPair;
use psl::{datamodel_connector::NativeTypeInstance, parser_database::ScalarType, ValidatedSchema};
use quaint::connector::DescribedQuery;
use schema_connector::{migrations_directory::MigrationDirectory, *};
use sql_doc_parser::{parse_sql_doc, sanitize_sql};
use sql_migration::{DropUserDefinedType, DropView, SqlMigration, SqlMigrationStep};
use sql_schema_describer as sql;
use std::{future, sync::Arc};

const MIGRATIONS_TABLE_NAME: &str = "_prisma_migrations";

/// The top-level SQL migration connector.
pub struct SqlSchemaConnector {
    flavour: Box<dyn SqlFlavour + Send + Sync + 'static>,
    host: Arc<dyn ConnectorHost>,
}

impl SqlSchemaConnector {
    /// Initialize an external PostgreSQL migration connector.
    #[cfg(all(feature = "postgresql", not(feature = "postgresql-native")))]
    pub async fn new_postgres_external(
        adapter: Arc<dyn quaint::connector::ExternalConnector>,
        factory: Arc<dyn quaint::connector::ExternalConnectorFactory>,
    ) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            flavour: Box::new(flavour::PostgresFlavour::new_external(adapter, factory).await?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize an external SQLite migration connector.
    #[cfg(all(feature = "sqlite", not(feature = "sqlite-native")))]
    pub fn new_sqlite_external(
        adapter: Arc<dyn quaint::connector::ExternalConnector>,
        factory: Arc<dyn quaint::connector::ExternalConnectorFactory>,
    ) -> Self {
        SqlSchemaConnector {
            flavour: Box::new(flavour::SqliteFlavour::new_external(adapter, factory)),
            host: Arc::new(EmptyHost),
        }
    }

    /// Initialize a PostgreSQL migration connector.
    #[cfg(feature = "postgresql-native")]
    pub fn new_postgres(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            flavour: Box::new(flavour::PostgresFlavour::new_postgres(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize a CockroachDb migration connector.
    #[cfg(feature = "cockroachdb-native")]
    pub fn new_cockroach(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            flavour: Box::new(flavour::PostgresFlavour::new_cockroach(params)?),
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
            flavour: Box::new(flavour::PostgresFlavour::new_with_params(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize a SQLite migration connector.
    #[cfg(feature = "sqlite-native")]
    pub fn new_sqlite(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            flavour: Box::new(flavour::SqliteFlavour::new_with_params(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize a MySQL migration connector.
    #[cfg(feature = "mysql-native")]
    pub fn new_mysql(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            flavour: Box::new(flavour::MysqlFlavour::new_with_params(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Initialize a MSSQL migration connector.
    #[cfg(feature = "mssql-native")]
    pub fn new_mssql(params: ConnectorParams) -> ConnectorResult<Self> {
        Ok(SqlSchemaConnector {
            flavour: Box::new(flavour::MssqlFlavour::new_with_params(params)?),
            host: Arc::new(EmptyHost),
        })
    }

    /// Create a new uninitialized MySQL migration connector.
    #[cfg(feature = "mysql-native")]
    pub fn new_uninitialized_mysql() -> Self {
        Self::new_uninitialized_flavour::<flavour::MysqlFlavour>()
    }

    /// Create a new uninitialized MSSQL migration connector.
    #[cfg(feature = "mssql-native")]
    pub fn new_uninitialized_mssql() -> Self {
        Self::new_uninitialized_flavour::<flavour::MssqlFlavour>()
    }

    /// Create a new uninitialized SQLite migration connector.
    #[cfg(feature = "sqlite-native")]
    pub fn new_uninitialized_sqlite() -> Self {
        Self::new_uninitialized_flavour::<flavour::SqliteFlavour>()
    }

    /// Create a new uninitialized PostgreSQL migration connector.
    #[cfg(feature = "postgresql-native")]
    pub fn new_uninitialized_postgres() -> Self {
        Self::new_uninitialized_flavour::<flavour::PostgresFlavour>()
    }

    /// Create a new uninitialized CockroachDB migration connector.
    #[cfg(feature = "cockroachdb-native")]
    pub fn new_uninitialized_cockroachdb() -> Self {
        Self {
            flavour: Box::new(flavour::PostgresFlavour::new_uninitialized_cockroach()),
            host: Arc::new(EmptyHost),
        }
    }

    fn new_uninitialized_flavour<Flavour: SqlFlavour + Default + 'static>() -> Self {
        Self {
            flavour: Box::new(Flavour::default()),
            host: Arc::new(EmptyHost),
        }
    }

    fn flavour(&self) -> &(dyn SqlFlavour + Send + Sync) {
        self.flavour.as_ref()
    }

    /// Made public for tests.
    pub fn describe_schema(
        &mut self,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'_, ConnectorResult<sql::SqlSchema>> {
        self.flavour.describe_schema(namespaces)
    }

    /// For tests
    pub async fn query_raw(
        &mut self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        self.flavour.query_raw(sql, params).await
    }

    /// For tests
    pub async fn query(
        &mut self,
        query: impl Into<quaint::ast::Query<'_>>,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        self.flavour.query(query.into()).await
    }

    /// For tests
    pub async fn raw_cmd(&mut self, sql: &str) -> ConnectorResult<()> {
        self.flavour.raw_cmd(sql).await
    }

    async fn db_schema_from_diff_target(
        &mut self,
        target: DiffTarget<'_>,
        shadow_database_connection_string: Option<String>,
        namespaces: Option<Namespaces>,
    ) -> ConnectorResult<SqlDatabaseSchema> {
        match target {
            DiffTarget::Datamodel(sources) => {
                let schema = psl::parse_schema_multi(&sources).map_err(ConnectorError::new_schema_parser_error)?;

                self.flavour.check_schema_features(&schema)?;
                Ok(sql_schema_calculator::calculate_sql_schema(
                    &schema,
                    self.flavour.as_ref(),
                ))
            }
            DiffTarget::Migrations(migrations) => self
                .flavour
                .sql_schema_from_migration_history(migrations, shadow_database_connection_string, namespaces)
                .await
                .map(From::from),
            DiffTarget::Database => self.flavour.describe_schema(namespaces).await.map(From::from),
            DiffTarget::Empty => Ok(self.flavour.empty_database_schema().into()),
        }
    }

    /// Returns the native types that can be used to represent the given scalar type.
    pub fn scalar_type_for_native_type(&self, native_type: &NativeTypeInstance) -> ScalarType {
        self.flavour
            .datamodel_connector()
            .scalar_type_for_native_type(native_type)
    }
}

impl SchemaConnector for SqlSchemaConnector {
    // TODO: this only seems to be used in `sql-migration-tests`.
    fn set_host(&mut self, host: Arc<dyn schema_connector::ConnectorHost>) {
        self.host = host;
    }

    fn set_preview_features(&mut self, preview_features: BitFlags<psl::PreviewFeature>) {
        self.flavour.set_preview_features(preview_features)
    }

    fn connector_type(&self) -> &'static str {
        self.flavour.connector_type()
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
        Box::pin(self.flavour.acquire_lock())
    }

    fn apply_migration<'a>(&'a mut self, migration: &'a Migration) -> BoxFuture<'a, ConnectorResult<u32>> {
        Box::pin(apply_migration::apply_migration(migration, self.flavour.as_mut()))
    }

    fn apply_script<'a>(&'a mut self, migration_name: &'a str, script: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        Box::pin(apply_migration::apply_script(migration_name, script, self))
    }

    fn empty_database_schema(&self) -> DatabaseSchema {
        DatabaseSchema::new(SqlDatabaseSchema::from(self.flavour.empty_database_schema()))
    }

    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.flavour.ensure_connection_validity()
    }

    // TODO: this only seems to be used in `sql-migration-tests`.
    fn host(&self) -> &Arc<dyn ConnectorHost> {
        &self.host
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        Box::pin(async {
            self.flavour
                .version()
                .await
                .map(|version| version.unwrap_or_else(|| "Database version information not available.".to_owned()))
        })
    }

    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>> {
        self.flavour.create_database()
    }

    fn database_schema_from_diff_target<'a>(
        &'a mut self,
        diff_target: DiffTarget<'a>,
        shadow_database_connection_string: Option<String>,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<DatabaseSchema>> {
        Box::pin(async move {
            self.db_schema_from_diff_target(diff_target, shadow_database_connection_string, namespaces)
                .await
                .map(From::from)
        })
    }

    fn db_execute(&mut self, script: String) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async move { self.flavour.raw_cmd(&script).await })
    }

    #[tracing::instrument(skip(self, from, to))]
    fn diff(&self, from: DatabaseSchema, to: DatabaseSchema) -> Migration {
        let previous = SqlDatabaseSchema::from_erased(from);
        let next = SqlDatabaseSchema::from_erased(to);
        let steps = sql_schema_differ::calculate_steps(MigrationPair::new(&previous, &next), self.flavour.as_ref());
        tracing::debug!(?steps, "Inferred migration steps.");

        Migration::new(SqlMigration {
            before: previous.describer_schema,
            after: next.describer_schema,
            steps,
        })
    }

    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>> {
        self.flavour.drop_database()
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
            let sql_schema = self.flavour.introspect(namespaces, ctx).await?;
            let search_path = self.flavour.search_path();
            let datamodel = introspection::datamodel_calculator::calculate(&sql_schema, ctx, search_path);

            Ok(datamodel)
        })
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
        apply_migration::render_script(migration, diagnostics, self.flavour())
    }

    fn reset(&mut self, soft: bool, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>> {
        Box::pin(async move {
            if soft || self.flavour.reset(namespaces.clone()).await.is_err() {
                best_effort_reset(self.flavour.as_mut(), namespaces).await?;
            }

            Ok(())
        })
    }

    fn migration_summary(&self, migration: &Migration) -> String {
        migration.downcast_ref::<SqlMigration>().drift_summary()
    }

    /// Optionally check that the features implied by the provided datamodel are all compatible with
    /// the specific database version being used.
    fn check_database_version_compatibility(
        &self,
        datamodel: &ValidatedSchema,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        self.flavour.check_database_version_compatibility(datamodel)
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
        migrations: &'a [MigrationDirectory],
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<()>> {
        Box::pin(async move {
            self.flavour
                .sql_schema_from_migration_history(migrations, None, namespaces)
                .await?;
            Ok(())
        })
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
            } = self.flavour.describe_query(&sanitized_sql).await?;
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
}

fn new_shadow_database_name() -> String {
    format!("prisma_migrate_shadow_db_{}", uuid::Uuid::new_v4())
}

/// Try to reset the database to an empty state. This should only be used
/// when we don't have the permissions to do a full reset.
#[tracing::instrument(skip(flavour))]
async fn best_effort_reset(
    flavour: &mut (dyn SqlFlavour + Send + Sync),
    namespaces: Option<Namespaces>,
) -> ConnectorResult<()> {
    best_effort_reset_impl(flavour, namespaces)
        .await
        .map_err(|err| err.into_soft_reset_failed_error())
}

async fn best_effort_reset_impl(
    flavour: &mut (dyn SqlFlavour + Send + Sync),
    namespaces: Option<Namespaces>,
) -> ConnectorResult<()> {
    tracing::info!("Attempting best_effort_reset");

    let source_schema = flavour.describe_schema(namespaces).await?;
    let target_schema = flavour.empty_database_schema();
    let mut steps = Vec::new();

    // We drop views here, not in the normal migration process to not
    // accidentally drop something we can't describe in the data model.
    let drop_views = source_schema
        .view_walkers()
        .filter(|view| !flavour.view_should_be_ignored(view.name()))
        .map(|vw| DropView::new(vw.id))
        .map(SqlMigrationStep::DropView);

    steps.extend(drop_views);

    let diffables: MigrationPair<SqlDatabaseSchema> = MigrationPair::new(source_schema, target_schema).map(From::from);
    steps.extend(sql_schema_differ::calculate_steps(diffables.as_ref(), flavour));
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
        flavour.drop_migrations_table().await?;
    }

    if migration.steps.is_empty() {
        return Ok(());
    }

    let migration = apply_migration::render_script(
        &Migration::new(migration),
        &DestructiveChangeDiagnostics::default(),
        flavour,
    )?;

    flavour.raw_cmd(&migration).await?;

    Ok(())
}
