#![deny(missing_docs)]

//! Migrations directory management.
//!
//! This module is responsible for the management of the contents of the
//! migrations directory. The migrations directory contains multiple migration
//! directorys, named after the migration id, and each containing:
//!
//! - A migration script

use sha2::{Digest, Sha256, Sha512};
use std::{
    error::Error,
    fmt::Display,
    fs::{read_dir, DirEntry},
    io::{self, Write as _},
    path::{Path, PathBuf},
};
use tracing_error::SpanTrace;

use crate::FormatChecksum;

/// The file name for migration scripts, not including the file extension.
pub const MIGRATION_SCRIPT_FILENAME: &str = "migration";

/// Create a directory for a new migration.
pub fn create_migration_directory(
    migrations_directory_path: &Path,
    migration_name: &str,
) -> io::Result<MigrationDirectory> {
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let directory_name = format!(
        "{timestamp}_{migration_name}",
        timestamp = timestamp,
        migration_name = migration_name
    );
    let directory_path = migrations_directory_path.join(directory_name);

    if directory_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            anyhow::anyhow!(
                "The migration directory already exists at {}",
                directory_path.to_string_lossy()
            ),
        ));
    }

    std::fs::create_dir_all(&directory_path)?;

    Ok(MigrationDirectory { path: directory_path })
}

/// An IO error that occurred while reading the migrations directory.
#[derive(Debug)]
pub struct ListMigrationsError(io::Error);

impl Display for ListMigrationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred when reading the migrations directory.")
    }
}

impl Error for ListMigrationsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl From<io::Error> for ListMigrationsError {
    fn from(err: io::Error) -> Self {
        ListMigrationsError(err)
    }
}

/// List the migrations present in the migration directory, lexicographically sorted by name.
///
/// If the migrations directory does not exist, it will not error but return an empty Vec.
pub fn list_migrations(migrations_directory_path: &Path) -> Result<Vec<MigrationDirectory>, ListMigrationsError> {
    let mut entries: Vec<MigrationDirectory> = Vec::new();

    let read_dir_entries = match read_dir(migrations_directory_path) {
        Ok(read_dir_entries) => read_dir_entries,
        Err(err) if matches!(err.kind(), std::io::ErrorKind::NotFound) => return Ok(entries),
        Err(err) => return Err(err.into()),
    };

    for entry in read_dir_entries {
        let entry = entry?;

        if entry.file_type()?.is_dir() {
            entries.push(entry.into());
        }
    }

    entries.sort_by(|a, b| a.migration_name().cmp(b.migration_name()));

    Ok(entries)
}

/// Proxy to a directory containing one migration, as returned by
/// `create_migration_directory` and `list_migrations`.
#[derive(Debug, Clone)]
pub struct MigrationDirectory {
    path: PathBuf,
}

#[derive(Debug)]
pub struct ReadMigrationScriptError(pub(crate) io::Error, pub(crate) SpanTrace);

impl Display for ReadMigrationScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to read migration script")
    }
}

impl From<io::Error> for ReadMigrationScriptError {
    fn from(err: io::Error) -> Self {
        ReadMigrationScriptError(err, SpanTrace::capture())
    }
}

impl Error for ReadMigrationScriptError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl MigrationDirectory {
    /// Initialize a MigrationDirectory at the provided path. This will not
    /// validate that the path is valid and exists.
    pub fn new(path: PathBuf) -> MigrationDirectory {
        MigrationDirectory { path }
    }

    /// The `{timestamp}_{name}` formatted migration name.
    pub fn migration_name(&self) -> &str {
        self.path
            .file_name()
            .expect("MigrationDirectory::migration_id")
            .to_str()
            .expect("Migration directory name is not valid UTF-8.")
    }

    /// Write the checksum of the migration script file to `buf`.
    pub fn checksum(&mut self, buf: &mut Vec<u8>) -> Result<(), ReadMigrationScriptError> {
        let script = self.read_migration_script()?;
        let mut hasher = Sha512::new();
        hasher.update(&script);
        let bytes = hasher.finalize();

        buf.clear();
        buf.extend_from_slice(bytes.as_ref());

        Ok(())
    }

    /// Check whether the checksum of the migration script matches the provided one.
    #[tracing::instrument]
    pub fn matches_checksum(&self, checksum_str: &str) -> Result<bool, ReadMigrationScriptError> {
        let filesystem_script = self.read_migration_script()?;
        let mut hasher = Sha256::new();
        hasher.update(&filesystem_script);
        let filesystem_script_checksum: [u8; 32] = hasher.finalize().into();

        Ok(checksum_str == filesystem_script_checksum.format_checksum())
    }

    /// Write the migration script to the directory.
    #[tracing::instrument]
    pub fn write_migration_script(&self, script: &str, extension: &str) -> std::io::Result<()> {
        let mut path = self.path.join(MIGRATION_SCRIPT_FILENAME);

        path.set_extension(extension);

        tracing::debug!("Writing migration script at {:?}", &path);

        let mut file = std::fs::File::create(&path)?;
        file.write_all(script.as_bytes())?;

        Ok(())
    }

    /// Read the migration script to a string.
    #[tracing::instrument]
    pub fn read_migration_script(&self) -> Result<String, ReadMigrationScriptError> {
        Ok(std::fs::read_to_string(&self.path.join("migration.sql"))?)
    }

    /// The filesystem path to the directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl From<DirEntry> for MigrationDirectory {
    fn from(entry: DirEntry) -> MigrationDirectory {
        MigrationDirectory { path: entry.path() }
    }
}
