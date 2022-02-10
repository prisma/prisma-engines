#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

//! This crate defines the API exposed by the connectors to the migration engine core. The entry point for this API is the [MigrationConnector](trait.MigrationConnector.html) trait.

mod connector_params;
mod checksum;
mod database_migration_step_applier;
mod destructive_change_checker;
mod diff;
mod error;
mod migration_persistence;

pub mod migrations_directory;

pub use connector_params::ConnectorParams;
pub use database_migration_step_applier::DatabaseMigrationStepApplier;
pub use destructive_change_checker::{
    DestructiveChangeChecker, DestructiveChangeDiagnostics, MigrationWarning, UnexecutableMigration,
};
pub use diff::DiffTarget;
pub use error::{ConnectorError, ConnectorResult};
pub use migration_persistence::{MigrationPersistence, MigrationRecord, PersistenceNotInitializedError, Timestamp};

use datamodel::ValidatedSchema;
use migrations_directory::MigrationDirectory;
use std::sync::Arc;

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

/// An abstract host for a migration connector. It exposes IO that is not directly performed by the
/// connectors.
#[async_trait::async_trait]
pub trait ConnectorHost: Sync + Send + 'static {
    /// Print to the console.
    async fn print(&self, text: &str) -> ConnectorResult<()>;
}

/// A no-op ConnectorHost.
#[derive(Debug, Clone)]
pub struct EmptyHost;

#[async_trait::async_trait]
impl ConnectorHost for EmptyHost {
    async fn print(&self, _text: &str) -> ConnectorResult<()> {
        Ok(())
    }
}

/// The top-level trait for connectors. This is the abstraction the migration engine core relies on to
/// interface with different database backends.
#[async_trait::async_trait]
pub trait MigrationConnector: Send + Sync + 'static {
    /// Accept a new ConnectorHost.
    fn set_host(&mut self, host: Arc<dyn ConnectorHost>);

    /// If possible on the target connector, acquire an advisory lock, so multiple instances of migrate do not run concurrently.
    async fn acquire_lock(&self) -> ConnectorResult<()>;

    /// A string that should identify what database backend is being used. Note that this is not necessarily
    /// the connector name. The SQL connector for example can return "postgresql", "mysql" or "sqlite".
    fn connector_type(&self) -> &'static str;

    /// Return the connection string that was used to initialize this connector.
    fn connection_string(&self) -> &str;

    /// Create the database referenced by Prisma schema that was used to initialize the connector.
    async fn create_database(&self) -> ConnectorResult<String>;

    /// Send a command to the database directly.
    async fn db_execute(&self, url: String, script: String) -> ConnectorResult<()>;

    /// Create a migration by comparing two database schemas. See
    /// [DiffTarget](/enum.DiffTarget.html) for possible inputs.
    async fn diff(&self, from: DiffTarget<'_>, to: DiffTarget<'_>) -> ConnectorResult<Migration>;

    /// Drop the database referenced by Prisma schema that was used to initialize the connector.
    async fn drop_database(&self) -> ConnectorResult<()>;

    /// Make sure the connection to the database is established and valid.
    /// Connectors can choose to connect lazily, but this method should force
    /// them to connect.
    async fn ensure_connection_validity(&self) -> ConnectorResult<()>;

    /// Return the ConnectorHost passed with set_host.
    fn host(&self) -> &Arc<dyn ConnectorHost>;

    /// The version of the underlying database.
    async fn version(&self) -> ConnectorResult<String>;

    /// Drop all database state.
    async fn reset(&self) -> ConnectorResult<()>;

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
    fn migration_persistence(&self) -> &dyn MigrationPersistence;

    /// Render a human-readable drift summary for the migration.
    fn migration_summary(&self, migration: &Migration) -> String;

    /// See [DatabaseMigrationStepApplier](trait.DatabaseMigrationStepApplier.html).
    fn database_migration_step_applier(&self) -> &dyn DatabaseMigrationStepApplier;

    /// See [DestructiveChangeChecker](trait.DestructiveChangeChecker.html).
    fn destructive_change_checker(&self) -> &dyn DestructiveChangeChecker;

    /// If possible, check that the passed in migrations apply cleanly.
    async fn validate_migrations(&self, _migrations: &[MigrationDirectory]) -> ConnectorResult<()>;
}
