#![deny(rust_2018_idioms)]

//! This crate defines the API exposed by the connectors to the migration engine core. The entry point for this API is the [MigrationConnector](trait.MigrationConnector.html) trait.

mod database_migration_inferrer;
mod database_migration_step_applier;
mod destructive_changes_checker;
mod error;
mod migration_applier;
mod migration_persistence;

pub mod steps;

pub use database_migration_inferrer::*;
pub use database_migration_step_applier::*;
pub use destructive_changes_checker::*;
pub use error::*;
pub use migration_applier::*;
pub use migration_persistence::*;
pub use steps::MigrationStep;

use std::fmt::Debug;
use user_facing_errors::migration_engine::DatabaseMigrationFormatChanged;

/// The top-level trait for connectors. This is the abstraction the migration engine core relies on to
/// interface with different database backends.
#[async_trait::async_trait]
pub trait MigrationConnector: Send + Sync + 'static {
    /// The data structure containing the concrete migration steps for the connector. A migration is
    /// assumed to consist of multiple steps.
    ///
    /// For example, in the SQL connector, a step would represent an SQL statement like `CREATE TABLE`.
    type DatabaseMigration: DatabaseMigrationMarker + Send + Sync + 'static;

    /// A string that should identify what database backend is being used. Note that this is not necessarily
    /// the connector name. The SQL connector for example can return "postgresql", "mysql" or "sqlite".
    fn connector_type(&self) -> &'static str;

    /// Hook to perform connector-specific initialization.
    async fn initialize(&self) -> ConnectorResult<()>;

    /// Create the database with the provided URL.
    async fn create_database(database_str: &str) -> ConnectorResult<String>;

    /// Drop all database state.
    async fn reset(&self) -> ConnectorResult<()>;

    /// Optionally check that the features implied by the provided datamodel are all compatible with
    /// the specific database version being used.
    fn check_database_version_compatibility(
        &self,
        _datamodel: &datamodel::dml::Datamodel,
    ) -> Vec<destructive_changes_checker::MigrationError> {
        Vec::new()
    }

    /// See [MigrationPersistence](trait.MigrationPersistence.html).
    fn migration_persistence<'a>(&'a self) -> Box<dyn MigrationPersistence + 'a>;

    /// See [DatabaseMigrationInferrer](trait.DatabaseMigrationInferrer.html).
    fn database_migration_inferrer<'a>(&'a self) -> Box<dyn DatabaseMigrationInferrer<Self::DatabaseMigration> + 'a>;

    /// See [DatabaseMigrationStepApplier](trait.DatabaseMigrationStepApplier.html).
    fn database_migration_step_applier<'a>(
        &'a self,
    ) -> Box<dyn DatabaseMigrationStepApplier<Self::DatabaseMigration> + 'a>;

    /// See [DestructiveChangesChecker](trait.DestructiveChangesChecker.html).
    fn destructive_changes_checker<'a>(&'a self) -> Box<dyn DestructiveChangesChecker<Self::DatabaseMigration> + 'a>;

    // TODO: figure out if this is the best way to do this or move to a better place/interface
    // this is placed here so i can use the associated type
    fn deserialize_database_migration(
        &self,
        json: serde_json::Value,
    ) -> Result<Self::DatabaseMigration, DatabaseMigrationFormatChanged>;

    /// See [MigrationStepApplier](trait.MigrationStepApplier.html).
    fn migration_applier<'a>(&'a self) -> Box<dyn MigrationApplier<Self::DatabaseMigration> + Send + Sync + 'a> {
        let applier = MigrationApplierImpl {
            migration_persistence: self.migration_persistence(),
            step_applier: self.database_migration_step_applier(),
        };
        Box::new(applier)
    }
}

pub trait DatabaseMigrationMarker: Debug + Send + Sync {
    fn serialize(&self) -> serde_json::Value;
}

/// Shorthand for a [Result](https://doc.rust-lang.org/std/result/enum.Result.html) where the error
/// variant is a [ConnectorError](/error/enum.ConnectorError.html).
pub type ConnectorResult<T> = Result<T, ConnectorError>;
