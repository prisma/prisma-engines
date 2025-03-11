use pretty_assertions::assert_eq;
use schema_core::{
    commands::create_migration, json_rpc::types::*, schema_connector::SchemaConnector, CoreError, CoreResult,
};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use test_setup::runtime::run_with_thread_local_runtime;

use crate::utils;

pub struct CreateMigration<'a> {
    api: &'a mut dyn SchemaConnector,
    files: Vec<SchemaContainer>,
    migrations_directory: &'a TempDir,
    draft: bool,
    name: &'a str,
}

impl<'a> CreateMigration<'a> {
    pub fn new(
        api: &'a mut dyn SchemaConnector,
        name: &'a str,
        files: &[(&'a str, &'a str)],
        migrations_directory: &'a TempDir,
    ) -> Self {
        CreateMigration {
            api,
            files: files
                .iter()
                .map(|(path, content)| SchemaContainer {
                    path: path.to_string(),
                    content: content.to_string(),
                })
                .collect(),
            migrations_directory,
            draft: false,
            name,
        }
    }

    pub fn draft(mut self, draft: bool) -> Self {
        self.draft = draft;

        self
    }

    pub async fn send(self) -> CoreResult<CreateMigrationAssertion<'a>> {
        // TODO: fix this
        let migrations_list = utils::list_migrations(self.migrations_directory.path()).unwrap();
        let migration_name = self.name.to_owned();
        let output = create_migration(
            CreateMigrationInput {
                migrations_list,
                schema: SchemasContainer { files: self.files },
                draft: self.draft,
                migration_name: migration_name.clone(),
            },
            self.api,
        )
        .await?;

        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let directory_name = format!("{timestamp}_{migration_name}");

        Ok(CreateMigrationAssertion {
            output,
            migrations_directory: self.migrations_directory,
            generated_migration_name: directory_name,
        })
    }

    #[track_caller]
    pub fn send_sync(self) -> CreateMigrationAssertion<'a> {
        run_with_thread_local_runtime(self.send()).unwrap()
    }

    #[track_caller]
    pub fn send_unwrap_err(self) -> CoreError {
        run_with_thread_local_runtime(self.send()).unwrap_err()
    }
}

pub struct CreateMigrationAssertion<'a> {
    pub output: CreateMigrationOutput,
    migrations_directory: &'a TempDir,
    generated_migration_name: String,
}

impl std::fmt::Debug for CreateMigrationAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CreateMigrationAssertion {{ .. }}")
    }
}

impl CreateMigrationAssertion<'_> {
    /// Assert that there are `expected_count` migrations in the migrations directory.
    /// TODO: this is currently failing.
    #[tracing::instrument(skip(self))]
    #[track_caller]
    pub fn assert_migration_directories_count(self, expected_count: usize) -> Self {
        let mut count = 0;

        for entry in
            std::fs::read_dir(self.migrations_directory.path()).expect("Counting directories in migrations directory.")
        {
            let entry = entry.unwrap();

            if entry.path().file_name().and_then(|s| s.to_str()) == Some("migration_lock.toml") {
                continue;
            }

            count += 1;
        }

        assert!(
            // the lock file is counted as an entry
            expected_count == count,
            "Assertion failed. Expected {expected_count} migrations in the migrations directory, found {count}."
        );

        self
    }

    /// Assert that there is one migration with `name_matcher` contained in its name present in the migration directory.
    pub fn assert_migration<F>(self, name_matcher: &str, assertions: F) -> Self
    where
        F: for<'b> FnOnce(MigrationAssertion<'b>) -> MigrationAssertion<'b>,
    {
        let migration = std::fs::read_dir(self.migrations_directory.path())
            .expect("Reading migrations directory for named migration.")
            .find_map(|entry| {
                let entry = entry.unwrap();
                let name = entry.file_name();

                if name.to_str().unwrap().contains(name_matcher) {
                    Some(entry)
                } else {
                    None
                }
            });

        match migration {
            Some(migration) => {
                let path = migration.path();
                let assertion = MigrationAssertion { path: path.as_ref() };

                assertions(assertion);
            }
            None => panic!("Assertion error. Could not find migration with name matching `{name_matcher}`"),
        }

        self
    }

    pub fn output(&self) -> &CreateMigrationOutput {
        &self.output
    }

    pub fn migration_script_path(&self) -> PathBuf {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let migration_name = &self.generated_migration_name;
        let directory_name = format!("{timestamp}_{migration_name}");

        self.migrations_directory
            .path()
            .join(directory_name)
            .join("migration.sql")
    }

    #[track_caller]
    pub fn modify_migration<F>(self, modify: F) -> Self
    where
        F: FnOnce(&mut String),
    {
        use std::io::Write as _;

        let migration_script_path = self.migration_script_path();
        let new_contents = {
            let mut contents = std::fs::read_to_string(&migration_script_path).expect("Reading migration script");

            modify(&mut contents);

            contents
        };

        let mut file = std::fs::File::create(&migration_script_path).unwrap();
        write!(file, "{new_contents}").unwrap();

        self
    }

    pub fn into_output(self) -> CreateMigrationOutput {
        self.output
    }
}

pub struct MigrationAssertion<'a> {
    path: &'a Path,
}

impl MigrationAssertion<'_> {
    #[track_caller]
    pub fn expect_contents(self, expected_contents: expect_test::Expect) -> Self {
        let migration_file_path = self.path.join("migration.sql");
        let contents: String = std::fs::read_to_string(&migration_file_path)
            .map_err(|_| format!("Trying to read migration file at {migration_file_path:?}"))
            .unwrap();

        expected_contents.assert_eq(&contents);
        self
    }

    #[track_caller]
    pub fn assert_contents(self, expected_contents: &str) -> Self {
        let migration_file_path = self.path.join("migration.sql");
        let contents: String = std::fs::read_to_string(&migration_file_path)
            .map_err(|_| format!("Trying to read migration file at {migration_file_path:?}"))
            .unwrap();

        assert_eq!(expected_contents, contents);
        self
    }
}
