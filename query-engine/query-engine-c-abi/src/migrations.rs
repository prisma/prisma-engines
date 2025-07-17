use indoc::indoc;
use query_engine_common::Result;
use query_engine_common::error::ApiError;
use rusqlite::Connection;
use std::{
    fs::{DirEntry, read_dir},
    path::{Path, PathBuf},
};

pub type Timestamp = chrono::DateTime<chrono::Utc>;

// TODO there is a bunch of casting that is present, however it is not the most correct way
// but since this is an out of tree branch, I do not want to change the common libraries yet

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

    /// Read the migration script to a string.
    pub fn read_migration_script(&self) -> Result<String> {
        let path = self.path.join("migration.sql");
        std::fs::read_to_string(path).map_err(|err| ApiError::Configuration(err.to_string()))
    }
}

impl From<DirEntry> for MigrationDirectory {
    fn from(entry: DirEntry) -> MigrationDirectory {
        MigrationDirectory { path: entry.path() }
    }
}

/// An applied migration, as returned by list_migrations.
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    /// A unique, randomly generated identifier.
    pub _id: String,
    /// The timestamp at which the migration completed *successfully*.
    pub finished_at: Option<Timestamp>,
    /// The name of the migration, i.e. the name of migration directory
    /// containing the migration script.
    pub migration_name: String,
    /// The time the migration started being applied.
    pub _started_at: Timestamp,
    /// The time the migration failed
    pub failed_at: Option<Timestamp>,
}

pub fn list_migration_dir(migrations_directory_path: &Path) -> Result<Vec<MigrationDirectory>> {
    let mut entries: Vec<MigrationDirectory> = Vec::new();

    let read_dir_entries = match read_dir(migrations_directory_path) {
        Ok(read_dir_entries) => read_dir_entries,
        Err(err) => return Err(ApiError::Configuration(err.to_string())),
    };

    for entry in read_dir_entries {
        let entry = entry.map_err(|err| ApiError::Configuration(err.to_string()))?;

        if entry
            .file_type()
            .map_err(|err| ApiError::Configuration(err.to_string()))?
            .is_dir()
        {
            entries.push(entry.into());
        }
    }

    entries.sort_by(|a, b| a.migration_name().cmp(b.migration_name()));

    Ok(entries)
}

pub fn detect_failed_migrations(migrations_from_database: &[MigrationRecord]) -> Result<()> {
    tracing::debug!("Checking for failed migrations.");

    let mut failed_migrations = migrations_from_database
        .iter()
        .filter(|migration| migration.finished_at.is_none() && migration.failed_at.is_none())
        .peekable();

    if failed_migrations.peek().is_none() {
        Ok(())
    } else {
        Err(ApiError::Configuration(
            format!(
                "Failed migration detected: {}",
                failed_migrations.peek().unwrap().migration_name
            )
            .to_string(),
        ))
    }
}

pub fn list_migrations(database_filename: &Path) -> Result<Vec<MigrationRecord>> {
    let conn = Connection::open(database_filename).map_err(|err| ApiError::Configuration(err.to_string()))?;

    // Check if the migrations table exists
    let table_exists = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='_prisma_migrations'")
        .and_then(|mut stmt| stmt.query_row([], |_| Ok(())))
        .is_ok();

    // If the migrations table doesn't exist, create it
    if !table_exists {
        let sql = indoc! {r#"
            CREATE TABLE "_prisma_migrations" (
                "id"                    TEXT PRIMARY KEY NOT NULL,
                "finished_at"           DATETIME,
                "migration_name"        TEXT NOT NULL,
                "started_at"            DATETIME NOT NULL DEFAULT current_timestamp,
                "failed_at"             DATETIME
            );
        "#};

        conn.execute(sql, [])
            .map_err(|err| ApiError::Configuration(err.to_string()))?;
    }

    let mut stmt = conn
        .prepare("SELECT id, migration_name, started_at, finished_at, failed_at FROM _prisma_migrations")
        .map_err(|err| ApiError::Configuration(err.to_string()))?;
    let mut rows = stmt.query([]).map_err(|err| ApiError::Configuration(err.to_string()))?;

    let mut entries: Vec<MigrationRecord> = Vec::new();

    while let Some(row) = rows.next().unwrap() {
        let id = row.get(0).unwrap();
        let migration_name: String = row.get(1).unwrap();
        let started_at: Timestamp = row.get(2).unwrap();
        let finished_at: Option<Timestamp> = row.get(3).unwrap();
        let failed_at: Option<Timestamp> = row.get(4).unwrap();

        entries.push(MigrationRecord {
            _id: id,
            migration_name,
            _started_at: started_at,
            finished_at,
            failed_at,
        });
    }

    Ok(entries)
}

pub fn record_migration_started(database_filename: &Path, migration_name: &str) -> Result<()> {
    let conn = Connection::open(database_filename).map_err(|err| ApiError::Configuration(err.to_string()))?;

    let sql = "INSERT INTO _prisma_migrations (id, migration_name) VALUES (?, ?)";
    conn.execute(sql, [uuid::Uuid::new_v4().to_string(), migration_name.to_owned()])
        .map_err(|err| ApiError::Configuration(err.to_string()))?;

    Ok(())
}

pub fn execute_migration_script(database_filename: &Path, migration_name: &str, script: &str) -> Result<()> {
    let conn = Connection::open(database_filename).map_err(|err| ApiError::Configuration(err.to_string()))?;

    let migration_result = conn.execute_batch(script);

    match migration_result {
        Ok(_) => {
            let sql = "UPDATE _prisma_migrations SET finished_at = current_timestamp WHERE migration_name = ?";
            conn.execute(sql, [migration_name])
                .map_err(|err| ApiError::Configuration(err.to_string()))?;
            Ok(())
        }
        Err(err) => {
            let sql = "UPDATE _prisma_migrations SET failed_at = current_timestamp WHERE migration_name = ?";
            conn.execute(sql, [migration_name])
                .map_err(|err| ApiError::Configuration(err.to_string()))?;
            Err(ApiError::Configuration(err.to_string()))
        }
    }
}
