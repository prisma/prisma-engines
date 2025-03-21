use std::sync::Arc;

use enumflags2::BitFlags;
use psl::ValidatedSchema;

use crate::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorHost, ConnectorResult, DatabaseSchema,
    DestructiveChangeChecker, DestructiveChangeDiagnostics, DiffTarget, IntrospectSqlQueryInput,
    IntrospectSqlQueryOutput, IntrospectionContext, IntrospectionResult, Migration, MigrationPersistence, Namespaces,
};

/// The top-level trait for connectors. This is the abstraction the schema engine core relies on to
/// interface with different database backends.
pub trait SchemaConnector: Send + Sync + 'static {
    // Setup methods

    /// Accept a new ConnectorHost.
    fn set_host(&mut self, host: Arc<dyn ConnectorHost>);

    /// Accept a new set of enabled preview features.
    fn set_preview_features(&mut self, preview_features: BitFlags<psl::PreviewFeature>);

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

    /// Create a migration by comparing two database schemas.
    fn diff(&self, from: DatabaseSchema, to: DatabaseSchema) -> Migration;

    /// Drop the database referenced by Prisma schema that was used to initialize the connector.
    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// An empty database schema (for diffing).
    fn empty_database_schema(&self) -> DatabaseSchema;

    /// Make sure the connection to the database is established and valid.
    /// Connectors can choose to connect lazily, but this method should force
    /// them to connect.
    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Return the ConnectorHost passed with set_host.
    fn host(&self) -> &Arc<dyn ConnectorHost>;

    /// The version of the underlying database.
    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<String>>;

    /// Render the migration to a runnable script.
    ///
    /// This should always return with `Ok` in normal circumstances. The result is currently only
    /// used to signal when the connector does not support rendering to a script.
    fn render_script(
        &self,
        migration: &Migration,
        diagnostics: &DestructiveChangeDiagnostics,
    ) -> ConnectorResult<String>;

    /// Drop all database state.
    ///
    /// Set the `soft` parameter to `true` to force a soft-reset, that is to say a reset that does
    /// not drop the database.
    fn reset(&mut self, soft: bool, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Optionally check that the features implied by the provided datamodel are all compatible with
    /// the specific database version being used.
    fn check_database_version_compatibility(
        &self,
        _datamodel: &ValidatedSchema,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        None
    }

    /// The file extension for generated migration files.
    fn migration_file_extension(&self) -> &'static str;

    /// Return whether the migration is empty.
    fn migration_is_empty(&self, migration: &Migration) -> bool {
        self.migration_len(migration) == 0
    }

    /// Return the number of steps in the migration.
    /// Invariant: migration_is_empty() == true iff migration_len() == 0.
    fn migration_len(&self, migration: &Migration) -> usize;

    /// See [MigrationPersistence](trait.MigrationPersistence.html).
    fn migration_persistence(&mut self) -> &mut dyn MigrationPersistence;

    /// Render a human-readable drift summary for the migration.
    fn migration_summary(&self, migration: &Migration) -> String;

    /// See [DestructiveChangeChecker](trait.DestructiveChangeChecker.html).
    fn destructive_change_checker(&mut self) -> &mut dyn DestructiveChangeChecker;

    /// Read a schema for diffing. The shadow database connection string is strictly optional, you
    /// don't need to pass it if a shadow database url was passed in params, or if it can be
    /// inferred from context, or if it isn't necessary for the task at hand.
    /// When MultiSchema is enabled, the namespaces are required for diffing anything other than a
    /// prisma schema, because that information is otherwise unavailable.
    fn database_schema_from_diff_target<'a>(
        &'a mut self,
        target: DiffTarget<'a>,
        shadow_database_connection_string: Option<String>,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<DatabaseSchema>>;

    /// In-tro-spec-shon.
    fn introspect<'a>(
        &'a mut self,
        ctx: &'a IntrospectionContext,
    ) -> BoxFuture<'a, ConnectorResult<IntrospectionResult>>;

    /// Introspect queries and returns type information.
    fn introspect_sql(
        &mut self,
        input: IntrospectSqlQueryInput,
    ) -> BoxFuture<'_, ConnectorResult<IntrospectSqlQueryOutput>>;

    /// If possible, check that the passed in migrations apply cleanly.
    fn validate_migrations<'a>(
        &'a mut self,
        _migrations: &'a [MigrationDirectory],
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Extract the namespaces from a Sql database schema (it will return None for mongodb).
    fn extract_namespaces(&self, schema: &DatabaseSchema) -> Option<Namespaces>;
}
