#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

//! This crate defines the API exposed by the connectors to the migration engine core. The entry point for this API is the [MigrationConnector](trait.MigrationConnector.html) trait.

mod checksum;
mod connector_params;
mod destructive_change_checker;
mod diff;
mod error;
mod migration_persistence;
mod namespaces;

pub mod migrations_directory;

pub use crate::namespaces::Namespaces;
pub use connector_params::ConnectorParams;
pub use destructive_change_checker::{
    DestructiveChangeChecker, DestructiveChangeDiagnostics, MigrationWarning, UnexecutableMigration,
};
pub use diff::DiffTarget;
use enumflags2::BitFlags;
pub use error::{ConnectorError, ConnectorResult};
pub use introspection_connector::{IntrospectionConnector, IntrospectionContext, IntrospectionResult};
pub use migration_persistence::{MigrationPersistence, MigrationRecord, PersistenceNotInitializedError, Timestamp};

use migrations_directory::MigrationDirectory;
use psl::ValidatedSchema;
use std::sync::Arc;

/// Alias for a pinned, boxed future, used by the traits.
pub type BoxFuture<'a, O> = std::pin::Pin<Box<dyn std::future::Future<Output = O> + Send + 'a>>;

/// A boxed migration, opaque to the migration engine core. The connectors are
/// sole responsible for producing and understanding migrations — the core just
/// orchestrates.
pub struct Migration(Box<dyn std::any::Any + Send + Sync>);

impl Migration {
    /// Type-erase a migration.
    pub fn new<T: 'static + Send + Sync>(migration: T) -> Self {
        Migration(Box::new(migration))
    }

    /// Should never be used in the core, only in connectors that know what they put there.
    pub fn downcast_ref<T: 'static>(&self) -> &T {
        self.0.downcast_ref().unwrap()
    }
}

/// A database schema. Part of the MigrationConnector API.
pub struct DatabaseSchema(Box<dyn std::any::Any + Send + Sync>);

impl DatabaseSchema {
    /// Type-erase a migration.
    pub fn new<T: 'static + Send + Sync>(migration: T) -> Self {
        DatabaseSchema(Box::new(migration))
    }

    /// Should never be used in the core, only in connectors that know what they put there.
    pub fn downcast<T: 'static>(self) -> Box<T> {
        self.0.downcast().unwrap()
    }

    /// Should never be used in the core, only in connectors that know what they put there.
    pub fn downcast_ref<T: 'static>(&self) -> &T {
        self.0.downcast_ref().unwrap()
    }
}

/// An abstract host for a migration connector. It exposes IO that is not directly performed by the
/// connectors.
pub trait ConnectorHost: Sync + Send + 'static {
    /// Print to the console.
    fn print<'a>(&'a self, text: &'a str) -> BoxFuture<'a, ConnectorResult<()>>;
}

/// A no-op ConnectorHost.
#[derive(Debug, Clone)]
pub struct EmptyHost;

impl ConnectorHost for EmptyHost {
    fn print(&self, text: &str) -> BoxFuture<'_, ConnectorResult<()>> {
        // https://github.com/prisma/prisma/issues/11761
        assert!(text.ends_with('\n'));
        Box::pin(std::future::ready(Ok(())))
    }
}

/// The top-level trait for connectors. This is the abstraction the migration engine core relies on to
/// interface with different database backends.
pub trait MigrationConnector: Send + Sync + 'static {
    // Setup methods

    /// Accept a new ConnectorHost.
    fn set_host(&mut self, host: Arc<dyn ConnectorHost>);

    /// Accept and validate new ConnectorParams. This should fail if it is called twice on the same
    /// connector.
    fn set_params(&mut self, params: ConnectorParams) -> ConnectorResult<()>;

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

    /// Return the connection string that was used to initialize this connector in set_params().
    fn connection_string(&self) -> Option<&str>;

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
        ctx: &'a introspection_connector::IntrospectionContext,
    ) -> BoxFuture<'a, ConnectorResult<introspection_connector::IntrospectionResult>>;

    /// If possible, check that the passed in migrations apply cleanly.
    fn validate_migrations<'a>(
        &'a mut self,
        _migrations: &'a [MigrationDirectory],
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Extract the namespaces from a Sql database schema (it will return None for mongodb).
    fn extract_namespaces(&self, schema: &DatabaseSchema) -> Option<Namespaces>;
}
