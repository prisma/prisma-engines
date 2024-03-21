use indoc::indoc;
use query_engine_common::error::ApiError;
use query_engine_common::Result;
use rusqlite::Connection;
use std::{
    fs::{read_dir, DirEntry},
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
    pub id: String,
    /// The timestamp at which the migration completed *successfully*.
    pub finished_at: Option<Timestamp>,
    /// The name of the migration, i.e. the name of migration directory
    /// containing the migration script.
    pub migration_name: String,
    /// The time the migration started being applied.
    pub started_at: Timestamp,
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

// pub fn detect_failed_migrations(migrations_from_database: &[MigrationRecord]) -> Result<(), user_facing_errors::Error> {
//     use std::fmt::Write as _;

//     tracing::debug!("Checking for failed migrations.");

//     let mut failed_migrations = migrations_from_database
//         .iter()
//         .filter(|migration| migration.finished_at.is_none() && migration.rolled_back_at.is_none())
//         .peekable();

//     if failed_migrations.peek().is_none() {
//         return Ok(());
//     }

//     let mut details = String::new();

//     for failed_migration in failed_migrations {
//         let logs = failed_migration
//             .logs
//             .as_deref()
//             .map(|s| s.trim())
//             .filter(|s| !s.is_empty())
//             .map(|s| format!(" with the following logs:\n{s}"))
//             .unwrap_or_default();

//         writeln!(
//             details,
//             "The `{name}` migration started at {started_at} failed{logs}",
//             name = failed_migration.migration_name,
//             started_at = failed_migration.started_at,
//         )
//         .unwrap();
//     }

//     // Err(user_facing(FoundFailedMigrations { details }))
//     Err(user_facing_errors::Error::from(
//         user_facing_errors::common::FoundFailedMigrations { details },
//     ))
// }

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
                "started_at"            DATETIME NOT NULL DEFAULT current_timestamp
            );
        "#};

        conn.execute(sql, [])
            .map_err(|err| ApiError::Configuration(err.to_string()))?;
    }

    let mut stmt = conn
        .prepare("SELECT id, migration_name, started_at, finished_at FROM _prisma_migrations")
        .map_err(|err| ApiError::Configuration(err.to_string()))?;
    let mut rows = stmt.query([]).map_err(|err| ApiError::Configuration(err.to_string()))?;

    let mut entries: Vec<MigrationRecord> = Vec::new();

    while let Some(row) = rows.next().unwrap() {
        let id = row.get(0).unwrap();
        let migration_name: String = row.get(1).unwrap();
        let started_at: Timestamp = row.get(2).unwrap();
        let finished_at: Option<Timestamp> = row.get(3).unwrap();

        entries.push(MigrationRecord {
            id,
            migration_name,
            started_at,
            finished_at,
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

    conn.execute_batch(script)
        .map_err(|err| ApiError::Configuration(err.to_string()))?;

    let sql = "UPDATE _prisma_migrations SET finished_at = current_timestamp WHERE migration_name = ?";
    conn.execute(sql, [migration_name])
        .map_err(|err| ApiError::Configuration(err.to_string()))?;

    Ok(())
}
