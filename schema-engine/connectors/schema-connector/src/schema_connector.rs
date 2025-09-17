use std::sync::Arc;

use psl::{PreviewFeatures, SourceFile, ValidatedSchema, parser_database::ExtensionTypes};
use quaint::connector::ExternalConnectorFactory;

use crate::{
    BoxFuture, ConnectorHost, ConnectorResult, DatabaseSchema, DestructiveChangeChecker, DestructiveChangeDiagnostics,
    DiffTarget, IntrospectSqlQueryInput, IntrospectSqlQueryOutput, IntrospectionContext, IntrospectionResult,
    Migration, MigrationPersistence, Namespaces, SchemaFilter, migrations_directory::Migrations,
};

/// The dialect for schema operations on a particular database.
pub trait SchemaDialect: Send + Sync + 'static {
    /// Create a migration by comparing two database schemas.
    fn diff(&self, from: DatabaseSchema, to: DatabaseSchema, filter: &SchemaFilter) -> Migration;

    /// Render the migration to a runnable script.
    ///
    /// This should always return with `Ok` in normal circumstances. The result is currently only
    /// used to signal when the connector does not support rendering to a script.
    fn render_script(
        &self,
        migration: &Migration,
        diagnostics: &DestructiveChangeDiagnostics,
    ) -> ConnectorResult<String>;

    /// The file extension for generated migration files.
    fn migration_file_extension(&self) -> &'static str;

    /// Return whether the migration is empty.
    fn migration_is_empty(&self, migration: &Migration) -> bool {
        self.migration_len(migration) == 0
    }

    /// Return the number of steps in the migration.
    /// Invariant: migration_is_empty() == true iff migration_len() == 0.
    fn migration_len(&self, migration: &Migration) -> usize;

    /// Render a human-readable drift summary for the migration.
    fn migration_summary(&self, migration: &Migration) -> String;

    /// Extract the namespaces from a Sql database schema (it will return None for mongodb).
    fn extract_namespaces(&self, schema: &DatabaseSchema) -> Option<Namespaces>;

    /// An empty database schema (for diffing).
    fn empty_database_schema(&self) -> DatabaseSchema;

    /// The default namespace for the dialect if it supports multiple namespaces.
    fn default_namespace(&self) -> Option<&str>;

    /// Create a database schema from datamodel source files.
    ///
    /// Note: The `default_namespace` should be taken from the connector's runtime
    /// configuration, which might be different from the dialect's default!
    fn schema_from_datamodel(
        &self,
        sources: Vec<(String, SourceFile)>,
        default_namespace: Option<&str>,
        extension_types: &dyn ExtensionTypes,
    ) -> ConnectorResult<DatabaseSchema>;

    /// If possible, check that the passed in migrations apply cleanly.
    fn validate_migrations_with_target<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
        target: ExternalShadowDatabase,
    ) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Create a database schema from migrations with a specific target shadow database.
    /// When MultiSchema is enabled, the namespaces are required for diffing anything other than a
    /// prisma schema, because that information is otherwise unavailable.
    fn schema_from_migrations_with_target<'a>(
        &'a self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
        target: ExternalShadowDatabase,
    ) -> BoxFuture<'a, ConnectorResult<DatabaseSchema>>;
}

/// The top-level trait for connectors. This is the abstraction the schema engine core relies on to
/// interface with different database backends.
pub trait SchemaConnector: Send + Sync + 'static {
    /// Return the schema dialect of the connector.
    fn schema_dialect(&self) -> Box<dyn SchemaDialect>;

    /// The default namespaces for the connector if it supports multiple namespaces.
    /// Should be derived from the connectors runtime configuration but can fallback to the dialect's default.
    fn default_runtime_namespace(&self) -> Option<&str>;

    /// Accept a new ConnectorHost.
    fn set_host(&mut self, host: Arc<dyn ConnectorHost>);

    /// Accept a new set of enabled preview features.
    fn set_preview_features(&mut self, preview_features: PreviewFeatures);

    // Connector methods

    /// If possible on the target connector, acquire an advisory lock, so multiple instances of migrate do not run concurrently.
    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Applies the migration to the database. Returns the number of executed steps.
    fn apply_migration<'a>(&'a mut self, migration: &'a Migration) -> BoxFuture<'a, ConnectorResult<u32>>;

    /// Apply a migration script to the database. The migration persistence is
    /// managed by the core.
    fn apply_script<'a>(&'a mut self, migration_name: &'a str, script: &'a str) -> BoxFuture<'a, ConnectorResult<()>>;

    /// A string that should identify what database backend is being used. Note that this is not necessarily
    /// the connector name. The SQL connector for example can return "postgresql", "mysql" or "sqlite".
    fn connector_type(&self) -> &'static str;

    /// Create the database referenced by Prisma schema that was used to initialize the connector.
    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>>;

    /// Send a command to the database directly.
    fn db_execute(&mut self, script: String) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Drop the database referenced by Prisma schema that was used to initialize the connector.
    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Make sure the connection to the database is established and valid.
    /// Connectors can choose to connect lazily, but this method should force
    /// them to connect.
    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Return the ConnectorHost passed with set_host.
    fn host(&self) -> &Arc<dyn ConnectorHost>;

    /// The version of the underlying database.
    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<String>>;

    /// Drop all database state.
    ///
    /// Set the `soft` parameter to `true` to force a soft-reset, that is to say a reset that does
    /// not drop the database.
    fn reset<'a>(
        &'a mut self,
        soft: bool,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
    ) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Optionally check that the features implied by the provided datamodel are all compatible with
    /// the specific database version being used.
    fn check_database_version_compatibility(
        &self,
        _datamodel: &ValidatedSchema,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        None
    }

    /// See [MigrationPersistence](trait.MigrationPersistence.html).
    fn migration_persistence(&mut self) -> &mut dyn MigrationPersistence;

    /// See [DestructiveChangeChecker](trait.DestructiveChangeChecker.html).
    fn destructive_change_checker(&mut self) -> &mut dyn DestructiveChangeChecker;

    /// Create a database schema from what's currently in the database.
    fn schema_from_database(
        &mut self,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'_, ConnectorResult<DatabaseSchema>>;

    /// Create a database schema from migrations using the shadow database configured in the
    /// connector.
    fn schema_from_migrations<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
    ) -> BoxFuture<'a, ConnectorResult<DatabaseSchema>>;

    /// In-tro-spec-shon.
    fn introspect<'a>(
        &'a mut self,
        ctx: &'a IntrospectionContext,
        extension_types: &'a dyn ExtensionTypes,
    ) -> BoxFuture<'a, ConnectorResult<IntrospectionResult>>;

    /// Introspect queries and returns type information.
    fn introspect_sql(
        &mut self,
        input: IntrospectSqlQueryInput,
    ) -> BoxFuture<'_, ConnectorResult<IntrospectSqlQueryOutput>>;

    /// If possible, check that the passed in migrations apply cleanly.
    fn validate_migrations<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
    ) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Read a schema for diffing.
    fn schema_from_diff_target<'a>(
        &'a mut self,
        diff_target: DiffTarget<'a>,
        namespaces: Option<Namespaces>,
        default_namespace: Option<&'a str>,
        filter: &'a SchemaFilter,
    ) -> BoxFuture<'a, ConnectorResult<DatabaseSchema>> {
        Box::pin(async move {
            match diff_target {
                DiffTarget::Datamodel(sources, extension_types) => {
                    self.schema_dialect()
                        .schema_from_datamodel(sources, default_namespace, extension_types)
                }
                DiffTarget::Migrations(migrations) => self.schema_from_migrations(migrations, namespaces, filter).await,
                DiffTarget::Database => self.schema_from_database(namespaces).await,
                DiffTarget::Empty => Ok(self.schema_dialect().empty_database_schema()),
            }
        })
    }

    /// Dispose of the connector.
    fn dispose(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;
}

#[derive(Clone)]
/// An external shadow database to be used for schema operations.
pub enum ExternalShadowDatabase {
    /// A driver adapter factory.
    DriverAdapter(Arc<dyn ExternalConnectorFactory>),
    /// A shadow database connection string and preview features.
    ConnectionString {
        /// The shadow database connection string.
        connection_string: String,
        /// The preview features.
        preview_features: PreviewFeatures,
    },
}
