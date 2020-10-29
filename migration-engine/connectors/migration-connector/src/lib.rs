#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

//! This crate defines the API exposed by the connectors to the migration engine core. The entry point for this API is the [MigrationConnector](trait.MigrationConnector.html) trait.

mod database_migration_inferrer;
mod database_migration_step_applier;
mod destructive_change_checker;
mod error;
mod imperative_migrations_persistence;
#[allow(missing_docs)]
mod migration_applier;
#[allow(missing_docs)]
mod migration_persistence;

#[allow(missing_docs)]
pub mod steps;

mod migrations_directory;

pub use database_migration_inferrer::*;
pub use database_migration_step_applier::*;
pub use destructive_change_checker::*;
pub use error::*;
pub use imperative_migrations_persistence::{
    ImperativeMigrationsPersistence, MigrationRecord, PersistenceNotInitializedError, Timestamp,
};
pub use migration_applier::*;
pub use migration_persistence::*;
pub use migrations_directory::{create_migration_directory, list_migrations, ListMigrationsError, MigrationDirectory};
pub use steps::MigrationStep;

use sha2::{Digest, Sha256};
use std::fmt::Debug;

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

    /// The version of the underlying database.
    fn version(&self) -> String;

    /// Hook to perform connector-specific initialization. This is deprecated.
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
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        None
    }

    /// See [MigrationPersistence](trait.MigrationPersistence.html).
    fn migration_persistence<'a>(&'a self) -> &dyn MigrationPersistence;

    /// See [ImperativeMigrationPersistence](trait.ImperativeMigrationPersistence.html).
    fn new_migration_persistence(&self) -> &dyn ImperativeMigrationsPersistence;

    /// See [DatabaseMigrationInferrer](trait.DatabaseMigrationInferrer.html).
    fn database_migration_inferrer(&self) -> &dyn DatabaseMigrationInferrer<Self::DatabaseMigration>;

    /// See [DatabaseMigrationStepApplier](trait.DatabaseMigrationStepApplier.html).
    fn database_migration_step_applier(&self) -> &dyn DatabaseMigrationStepApplier<Self::DatabaseMigration>;

    /// See [DestructiveChangeChecker](trait.DestructiveChangeChecker.html).
    fn destructive_change_checker(&self) -> &dyn DestructiveChangeChecker<Self::DatabaseMigration>;

    /// See [MigrationStepApplier](trait.MigrationStepApplier.html).
    fn migration_applier<'a>(&'a self) -> Box<dyn MigrationApplier<Self::DatabaseMigration> + Send + Sync + 'a> {
        let applier = MigrationApplierImpl {
            migration_persistence: self.migration_persistence(),
            step_applier: self.database_migration_step_applier(),
        };
        Box::new(applier)
    }
}

/// Marker for the associated migration type for a connector.
pub trait DatabaseMigrationMarker: Debug + Send + Sync {
    /// The file extension to use for migration scripts.
    const FILE_EXTENSION: &'static str;

    /// Render the migration as JSON.
    fn serialize(&self) -> serde_json::Value;

    /// Is the migration empty?
    fn is_empty(&self) -> bool;
}

/// Shorthand for a [Result](https://doc.rust-lang.org/std/result/enum.Result.html) where the error
/// variant is a [ConnectorError](/error/enum.ConnectorError.html).
pub type ConnectorResult<T> = Result<T, ConnectorError>;

/// Compute the checksum for a migration script, and return it formatted to be human-readable.
fn checksum(script: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(script.as_bytes());
    let checksum: [u8; 32] = hasher.finalize().into();
    checksum.format_checksum()
}

/// Format a checksum to a hexadecimal string. This is used to checksum
/// migration scripts with Sha256.
pub trait FormatChecksum {
    /// Format a checksum to a hexadecimal string.
    fn format_checksum(&self) -> String;
}

impl FormatChecksum for [u8; 32] {
    fn format_checksum(&self) -> String {
        use std::fmt::Write as _;

        let mut checksum_string = String::with_capacity(32 * 2);

        for byte in self {
            write!(checksum_string, "{:x}", byte).unwrap();
        }

        checksum_string
    }
}
