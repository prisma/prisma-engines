#![deny(missing_docs)]
#![allow(dead_code)]

//! Migrations directory management.
//!
//! This module is responsible for the management of the contents of the
//! migrations directory. The migrations directory contains multiple migration
//! directorys, named after the migration id, and each containing:
//!
//! - A migration script

// use migration_connector::ImperativeMigration;
use sha2::{Digest, Sha512};
use std::{
    fs::{create_dir, read_dir, DirEntry},
    io::{self, Write as _},
    path::{Path, PathBuf},
};

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

    create_dir(&directory_path)?;

    Ok(MigrationDirectory { path: directory_path })
}

/// List the migrations present in the migration directory, lexicographically sorted by name.
pub fn list_migrations(migrations_directory_path: &Path) -> io::Result<Vec<MigrationDirectory>> {
    let mut entries: Vec<MigrationDirectory> = Vec::new();

    for entry in read_dir(migrations_directory_path)? {
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
#[derive(Debug)]
pub struct MigrationDirectory {
    path: PathBuf,
}

impl MigrationDirectory {
    /// The `{timestamp}_{name}` formatted migration name.
    pub fn migration_name(&self) -> &str {
        self.path
            .file_name()
            .expect("MigrationDirectory::migration_id")
            .to_str()
            .expect("Migration directory name is not valid UTF-8.")
    }

    /// Write the checksum of the migration script file to `buf`.
    pub fn checksum(&mut self, buf: &mut Vec<u8>) -> io::Result<()> {
        let script = self.read_migration_script()?;
        let mut hasher = Sha512::new();
        hasher.update(&script);
        let bytes = hasher.finalize();

        buf.clear();
        buf.extend_from_slice(bytes.as_ref());

        Ok(())
    }

    // #[tracing::instrument]
    // pub fn matches_applied_migration(&self, applied_migration: &ImperativeMigration) -> io::Result<bool> {
    //     let filesystem_script = self.read_migration_script()?;
    //     let mut hasher = Sha512::new();
    //     hasher.update(&filesystem_script);
    //     let filesystem_script_checksum = hasher.finalize();

    //     Ok(applied_migration.checksum == filesystem_script_checksum.as_ref())
    // }

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
    pub fn read_migration_script(&self) -> std::io::Result<String> {
        std::fs::read_to_string(&self.path.join("migration.sql"))
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
