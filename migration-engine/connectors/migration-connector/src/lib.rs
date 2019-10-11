//! This crate defines the API exposed by the connectors to the migration engine core. The entry point for this API is the [MigrationConnector](trait.MigrationConnector.html) trait.

mod database_migration_inferrer;
mod database_migration_step_applier;
mod destructive_changes_checker;
mod error;
mod migration_applier;
mod migration_persistence;

pub mod ast_steps;
pub mod steps;

pub use database_migration_inferrer::*;
pub use database_migration_step_applier::*;
pub use destructive_changes_checker::*;
pub use error::*;
pub use migration_applier::*;
pub use migration_persistence::*;
pub use steps::MigrationStep;

use std::fmt::Debug;
use std::sync::Arc;

/// The top-level trait for connectors. This is the abstraction the migration engine core relies on to
/// interface with different database backends.
pub trait MigrationConnector: Send + Sync + 'static {
    /// The data structure containing the concrete migration steps for the connector. A migration is
    /// assumed to consist of multiple steps.
    ///
    /// For example, in the SQL connector, a step would represent an SQL statement like `CREATE TABLE`.
    type DatabaseMigration: DatabaseMigrationMarker + 'static;

    /// A string that should identify what database backend is being used. Note that this is not necessarily
    /// the connector name. The SQL connector for example can return "postgresql", "mysql" or "sqlite".
    fn connector_type(&self) -> &'static str;

    /// Create a new database with the passed in name.
    fn create_database(&self, create: &str) -> ConnectorResult<()>;

    /// Hook to perform connector-specific initialization.
    fn initialize(&self) -> ConnectorResult<()>;

    /// Drop all database state.
    fn reset(&self) -> ConnectorResult<()>;

    /// See [MigrationPersistence](trait.MigrationPersistencey.html).
    fn migration_persistence(&self) -> Arc<dyn MigrationPersistence>;

    /// See [DatabaseMigrationInferrer](trait.DatabaseMigrationInferrer.html).
    fn database_migration_inferrer(&self) -> Arc<dyn DatabaseMigrationInferrer<Self::DatabaseMigration>>;

    /// See [DatabaseMigrationStepApplier](trait.DatabaseMigrationStepApplier.html).
    fn database_migration_step_applier(&self) -> Arc<dyn DatabaseMigrationStepApplier<Self::DatabaseMigration>>;

    /// See [DestructiveChangesChecker](trait.DestructiveChangesChecker.html).
    fn destructive_changes_checker(&self) -> Arc<dyn DestructiveChangesChecker<Self::DatabaseMigration>>;

    // TODO: figure out if this is the best way to do this or move to a better place/interface
    // this is placed here so i can use the associated type
    fn deserialize_database_migration(&self, json: serde_json::Value) -> Self::DatabaseMigration;

    /// See [MigrationStepApplier](trait.MigrationStepApplier.html).
    fn migration_applier(&self) -> Box<dyn MigrationApplier<Self::DatabaseMigration>> {
        let applier = MigrationApplierImpl {
            migration_persistence: self.migration_persistence(),
            step_applier: self.database_migration_step_applier(),
        };
        Box::new(applier)
    }
}

pub trait DatabaseMigrationMarker: Debug {
    fn serialize(&self) -> serde_json::Value;
}

pub type ConnectorResult<T> = Result<T, ConnectorError>;
