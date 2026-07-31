use std::{
    error::Error,
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::Path,
};

use quaint::{prelude::Queryable, single::Quaint};
use schema_core::json_rpc::types::{
    MigrationDirectory, MigrationFile, MigrationList, MigrationLockfile, SchemaContainer,
};
use test_setup::{Tags, TestApiArgs, runtime::run_with_thread_local_runtime as tok};

#[macro_export]
macro_rules! write_multi_file {
  // Match multiple pairs of filename and content
  ( $( $filename:expr => $content:expr ),* $(,)? ) => {
      {
          use std::fs::File;
          use std::io::Write;

          // Create a result vector to collect errors
          let mut results = Vec::new();
          let tmpdir = tempfile::tempdir().unwrap();

          std::fs::create_dir_all(&tmpdir).unwrap();

          $(
              let file_path = tmpdir.path().join($filename);
              // Attempt to create or open the file
              let result = (|| -> std::io::Result<()> {
                  let mut file = File::create(&file_path)?;
                  file.write_all($content.as_bytes())?;
                  Ok(())
              })();

              result.unwrap();

              results.push((file_path.to_string_lossy().into_owned(), $content));
          )*

          (tmpdir, results)
      }
  };
}

pub fn to_schema_containers(files: &[(String, &str)]) -> Vec<SchemaContainer> {
    files
        .iter()
        .map(|(path, content)| SchemaContainer {
            path: path.to_string(),
            content: content.to_string(),
        })
        .collect()
}

/// List the migrations present in the migration directory, lexicographically sorted by name.
///
/// If the migrations directory does not exist, it will not error but return an empty Vec.
pub fn list_migrations(migrations_directory_path: &Path) -> Result<MigrationList, ListMigrationsError> {
    let base_dir = migrations_directory_path.to_string_lossy().into_owned();

    let lockfile = MigrationLockfile {
        path: "migration_lock.toml".to_string(),
        content: std::fs::read_to_string(migrations_directory_path.join("migration_lock.toml")).ok(),
    };

    let mut entries: Vec<MigrationDirectory> = Vec::new();

    let read_dir_entries = match std::fs::read_dir(migrations_directory_path) {
        Ok(read_dir_entries) => read_dir_entries,
        Err(err) if matches!(err.kind(), std::io::ErrorKind::NotFound) => {
            return Ok(MigrationList {
                base_dir,
                lockfile,
                shadow_db_init_script: Default::default(),
                migration_directories: entries,
            });
        }
        Err(err) => return Err(err.into()),
    };

    for entry in read_dir_entries {
        let entry = entry?;

        if entry.file_type()?.is_dir() {
            let entry = entry.path();

            // Relative path to a migration directory from `baseDir`.
            // E.g., `20201117144659_test`.
            // This will return a &Path that is the relative path
            let entry_relative = entry.strip_prefix(&base_dir).expect("entry is not inside base_dir");

            let path = entry_relative.to_string_lossy().into_owned();

            let migration_file = MigrationFile {
                path: "migration.sql".to_string(),
                content: std::fs::read_to_string(entry.join("migration.sql"))
                    .map_err(|_err| "Could not read migration file.".to_owned())
                    .into(),
            };

            let migration_directory = MigrationDirectory { path, migration_file };
            entries.push(migration_directory);
        }
    }

    entries.sort_by(|a, b| a.migration_name().cmp(b.migration_name()));

    Ok(MigrationList {
        base_dir,
        lockfile,
        shadow_db_init_script: Default::default(),
        migration_directories: entries,
    })
}

/// An IO error that occurred while reading the migrations directory.
#[derive(Debug)]
pub struct ListMigrationsError(io::Error);

impl Display for ListMigrationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("An error occurred when reading the migrations directory.")
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

/// Creates a database on the same server as the test database, and returns a connection string for
/// it. Commands that replay a migration history need a shadow database that is not the database
/// they are looking at, and this is the second database on the server that satisfies them.
pub fn create_external_shadow_database(args: &TestApiArgs) -> String {
    let name = shadow_database_name(args.test_function_name());
    let tags = args.tags();

    if tags.contains(Tags::Postgres) {
        tok(test_setup::postgres::create_postgres_database(
            args.database_url(),
            &name,
        ))
        .unwrap()
        .1
    } else if tags.contains(Tags::Mysql) {
        tok(test_setup::mysql::create_mysql_database(args.database_url(), &name))
            .unwrap()
            .1
    } else if tags.contains(Tags::Mssql) {
        tok(test_setup::mssql::init_mssql_database(args.database_url(), &name))
            .unwrap()
            .1
    } else if tags.contains(Tags::Sqlite) {
        test_setup::sqlite_test_url(&name)
    } else {
        panic!("No external shadow database for the database under test.")
    }
}

/// Database names are limited to 63 bytes on PostgreSQL and to 64 on MySQL, while test function
/// names alone can be longer than that, and truncation alone makes the names of tests that share a
/// long prefix or suffix collide. A hash of the full name keeps them apart.
fn shadow_database_name(test_function_name: &str) -> String {
    let mut hasher = DefaultHasher::new();
    test_function_name.hash(&mut hasher);
    let hash = hasher.finish() as u32;
    let prefix: String = test_function_name.chars().take(40).collect();

    format!("{prefix}_{hash:x}_shadow")
}

/// Runs a SQL command against an arbitrary database, on a connection of its own. Used to set up and
/// to inspect databases the engine under test is not connected to, such as a shadow database.
pub fn raw_cmd_on(connection_string: &str, sql: &str) {
    tok(async {
        let connection = Quaint::new(connection_string).await.unwrap();
        connection.raw_cmd(sql).await.unwrap();
    })
}

/// Runs a SQL query against an arbitrary database, on a connection of its own, and returns what it
/// answered. The counterpart of [`raw_cmd_on`] for the cases where the answer is the point.
pub fn query_on(connection_string: &str, sql: &str) -> quaint::prelude::ResultSet {
    tok(async {
        let connection = Quaint::new(connection_string).await.unwrap();
        connection.query_raw(sql, &[]).await.unwrap()
    })
}
