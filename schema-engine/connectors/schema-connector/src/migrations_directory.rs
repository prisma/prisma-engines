//! Migrations directory management.
//!
//! This module is responsible for the management of the contents of the
//! migrations directory. At the top level it contains a migration_lock.toml file which lists the provider.
//! It also contains multiple subfolders, named after the migration id, and each containing:
//! - A migration script

use crate::{checksum, ConnectorError, ConnectorResult};
use json_rpc::types::MigrationLockfile;
use std::{error::Error, fmt::Display, hash};
use tracing_error::SpanTrace;
use user_facing_errors::schema_engine::ProviderSwitchedError;

/// The file name for migration scripts, not including the file extension.
pub const MIGRATION_SCRIPT_FILENAME: &str = "migration";

/// The file name for the migration lock file, not including the file extension.
pub const MIGRATION_LOCK_FILENAME: &str = "migration_lock";

/// Error if the provider in the schema does not match the one in the schema_lock.toml
pub fn error_on_changed_provider(lockfile: &MigrationLockfile, provider: &str) -> ConnectorResult<()> {
    match match_provider_in_lock_file(lockfile, provider) {
        None => Ok(()),
        Some(Err(expected_provider)) => Err(ConnectorError::user_facing(ProviderSwitchedError {
            provider: provider.into(),
            expected_provider,
        })),
        Some(Ok(())) => Ok(()),
    }
}

/// Check whether provider matches. `None` means there was no migration_lock.toml file.
fn match_provider_in_lock_file(lockfile: &MigrationLockfile, provider: &str) -> Option<Result<(), String>> {
    read_provider_from_lock_file(lockfile).map(|found_provider| {
        if found_provider == provider {
            Ok(())
        } else {
            Err(found_provider)
        }
    })
}

/// Read the provider from the migration_lock.toml. `None` means there was no migration_lock.toml
/// file in the directory.
pub fn read_provider_from_lock_file(lockfile: &MigrationLockfile) -> Option<String> {
    lockfile.content.as_ref().map(|content| {
        content
            .lines()
            .find(|line| line.starts_with("provider"))
            .map(|line| line.trim_start_matches("provider = ").trim_matches('"'))
            .unwrap_or("<PROVIDER NOT FOUND>")
            .to_owned()
    })
}

/// Returns a list of migration directories from the filesystem, with extra functionality.
pub fn list_migrations(
    migrations_from_filesystem: Vec<json_rpc::types::MigrationDirectory>,
) -> Vec<MigrationDirectory> {
    migrations_from_filesystem
        .into_iter()
        .map(MigrationDirectory::new)
        .collect()
}

/// Proxy to a directory containing one migration.
#[derive(Debug, Clone)]
pub struct MigrationDirectory(pub(crate) json_rpc::types::MigrationDirectory);

impl MigrationDirectory {
    /// Create a new migration directory proxy.
    pub fn new(dir: json_rpc::types::MigrationDirectory) -> Self {
        Self(dir)
    }

    /// The `{timestamp}_{name}` formatted migration name.
    pub fn migration_name(&self) -> &str {
        self.0.migration_name()
    }

    /// Check whether the checksum of the migration script matches the provided one.
    /// TODO: reduce clone usage here.
    pub fn matches_checksum(&self, checksum_str: &str) -> Result<bool, ReadMigrationScriptError> {
        let filesystem_script = self.read_migration_script()?;
        Ok(checksum::script_matches_checksum(&filesystem_script, checksum_str))
    }

    /// Read the migration script to a string.
    pub fn read_migration_script(&self) -> Result<String, ReadMigrationScriptError> {
        let migration_file_path = self.0.migration_file.path.clone();
        let filesystem_script: Result<String, String> = self.0.migration_file.content.clone().into();

        filesystem_script.map_err(|err| ReadMigrationScriptError::new(std::io::Error::other(err), migration_file_path))
    }
}

impl hash::Hash for MigrationDirectory {
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        self.0.hash(hasher)
    }
}

/// Error while reading a migration script.
#[derive(Debug)]
pub struct ReadMigrationScriptError(pub(crate) std::io::Error, pub SpanTrace, pub String);

impl ReadMigrationScriptError {
    fn new(err: std::io::Error, file_path: String) -> Self {
        ReadMigrationScriptError(err, SpanTrace::capture(), file_path)
    }
}

impl Display for ReadMigrationScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Failed to read migration script at ")?;
        Display::fmt(&self.2, f)
    }
}

impl Error for ReadMigrationScriptError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}
