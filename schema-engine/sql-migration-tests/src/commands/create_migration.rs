use pretty_assertions::assert_eq;
use schema_core::{
    commands::create_migration, json_rpc::types::*, schema_connector::SchemaConnector, CoreError, CoreResult,
};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use test_setup::runtime::run_with_thread_local_runtime;

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
        let output = create_migration(
            CreateMigrationInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
                schema: SchemasContainer { files: self.files },
                draft: self.draft,
                migration_name: self.name.to_owned(),
            },
            self.api,
        )
        .await?;

        Ok(CreateMigrationAssertion {
            output,
            migrations_directory: self.migrations_directory,
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
}

impl std::fmt::Debug for CreateMigrationAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CreateMigrationAssertion {{ .. }}")
    }
}

impl<'a> CreateMigrationAssertion<'a> {
    /// Assert that there are `expected_count` migrations in the migrations directory.
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
        self.migrations_directory
            .path()
            .join(self.output.generated_migration_name.as_ref().unwrap())
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
