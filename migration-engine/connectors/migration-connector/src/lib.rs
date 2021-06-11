#![deny(rust_2018_idioms, unsafe_code, missing_docs)]

//! This crate defines the API exposed by the connectors to the migration engine core. The entry point for this API is the [MigrationConnector](trait.MigrationConnector.html) trait.

mod database_migration_step_applier;
mod destructive_change_checker;
mod diff;
mod error;
mod migration_persistence;

pub mod migrations_directory;

pub use database_migration_step_applier::DatabaseMigrationStepApplier;
pub use destructive_change_checker::{
    DestructiveChangeChecker, DestructiveChangeDiagnostics, MigrationWarning, UnexecutableMigration,
};
pub use diff::DiffTarget;
pub use error::ConnectorError;
pub use migration_persistence::{MigrationPersistence, MigrationRecord, PersistenceNotInitializedError, Timestamp};

use migrations_directory::MigrationDirectory;
use sha2::{Digest, Sha256};

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

/// The top-level trait for connectors. This is the abstraction the migration engine core relies on to
/// interface with different database backends.
#[async_trait::async_trait]
pub trait MigrationConnector: Send + Sync + 'static {
    /// If possible on the target connector, acquire an advisory lock, so multiple instances of migrate do not run concurrently.
    async fn acquire_lock(&self) -> ConnectorResult<()>;

    /// A string that should identify what database backend is being used. Note that this is not necessarily
    /// the connector name. The SQL connector for example can return "postgresql", "mysql" or "sqlite".
    fn connector_type(&self) -> &'static str;

    /// Create a migration by comparing two database schemas. See
    /// [DiffTarget](/enum.DiffTarget.html) for possible inputs.
    async fn diff(&self, from: DiffTarget<'_>, to: DiffTarget<'_>) -> ConnectorResult<Migration>;

    /// The version of the underlying database.
    async fn version(&self) -> ConnectorResult<String>;

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

/// The length (in bytes, or equivalently ascii characters) of the checksum
/// strings.
const CHECKSUM_STR_LEN: usize = 64;

/// Format a checksum to a hexadecimal string. This is used to checksum
/// migration scripts with Sha256.
trait FormatChecksum {
    /// Format a checksum to a hexadecimal string.
    fn format_checksum(&self) -> String;
    /// Obsolete checksum method, should only be used for compatibility.
    fn format_checksum_old(&self) -> String;
}

impl FormatChecksum for [u8; 32] {
    fn format_checksum(&self) -> String {
        use std::fmt::Write as _;

        let mut checksum_string = String::with_capacity(32 * 2);

        for byte in self {
            write!(checksum_string, "{:02x}", byte).unwrap();
        }

        assert_eq!(checksum_string.len(), CHECKSUM_STR_LEN);

        checksum_string
    }

    // Due to an omission in a previous version of the migration engine,
    // some migrations tables will have old migrations with checksum strings
    // that have not been zero-padded.
    //
    // Corresponding issue:
    // https://github.com/prisma/prisma-engines/issues/1887
    fn format_checksum_old(&self) -> String {
        use std::fmt::Write as _;

        let mut checksum_string = String::with_capacity(32 * 2);

        for byte in self {
            write!(checksum_string, "{:x}", byte).unwrap();
        }

        checksum_string
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_checksum_does_not_strip_zeros() {
        assert_eq!(
            checksum("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(checksum("abcd").len(), CHECKSUM_STR_LEN);
    }
}
