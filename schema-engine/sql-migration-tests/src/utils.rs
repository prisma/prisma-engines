use std::{error::Error, fmt::Display, io, path::Path};

use schema_core::json_rpc::types::{
    MigrationDirectory, MigrationFile, MigrationList, MigrationLockfile, SchemaContainer,
};

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
