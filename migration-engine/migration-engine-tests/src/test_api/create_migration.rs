use anyhow::Context;
use migration_core::{
    commands::CreateMigrationInput, commands::CreateMigrationOutput, CoreError, CoreResult, GenericApi,
};
use pretty_assertions::assert_eq;
use std::path::Path;
use tempfile::TempDir;

use crate::AssertionResult;

pub struct CreateMigration<'a> {
    api: &'a dyn GenericApi,
    schema: &'a str,
    migrations_directory: &'a TempDir,
    draft: bool,
    name: &'a str,
    rt: Option<&'a tokio::runtime::Runtime>,
}

impl<'a> CreateMigration<'a> {
    pub fn new(api: &'a dyn GenericApi, name: &'a str, schema: &'a str, migrations_directory: &'a TempDir) -> Self {
        CreateMigration {
            api,
            schema,
            migrations_directory,
            draft: false,
            name,
            rt: None,
        }
    }

    pub fn new_sync(
        api: &'a dyn GenericApi,
        name: &'a str,
        schema: &'a str,
        migrations_directory: &'a TempDir,
        rt: &'a tokio::runtime::Runtime,
    ) -> Self {
        let mut initial = Self::new(api, name, schema, migrations_directory);
        initial.rt = Some(rt);
        initial
    }

    pub fn draft(mut self, draft: bool) -> Self {
        self.draft = draft;

        self
    }

    pub async fn send(self) -> CoreResult<CreateMigrationAssertion<'a>> {
        let output = self
            .api
            .create_migration(&CreateMigrationInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
                prisma_schema: self.schema.to_owned(),
                draft: self.draft,
                migration_name: self.name.to_owned(),
            })
            .await?;

        Ok(CreateMigrationAssertion {
            output,
            _api: self.api,
            migrations_directory: self.migrations_directory,
        })
    }

    #[track_caller]
    pub fn send_sync(self) -> CreateMigrationAssertion<'a> {
        self.rt.unwrap().block_on(self.send()).unwrap()
    }

    #[track_caller]
    pub fn send_unwrap_err(self) -> CoreError {
        self.rt.unwrap().block_on(self.send()).unwrap_err()
    }
}

pub struct CreateMigrationAssertion<'a> {
    output: CreateMigrationOutput,
    _api: &'a dyn GenericApi,
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

        for entry in std::fs::read_dir(self.migrations_directory.path())
            .context("Counting directories in migrations directory.")
            .unwrap()
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
            "Assertion failed. Expected {expected} migrations in the migrations directory, found {actual}.",
            expected = expected_count,
            actual = count
        );

        self
    }

    /// Assert that there is one migration with `name_matcher` contained in its name present in the migration directory.
    pub fn assert_migration<F>(self, name_matcher: &str, assertions: F) -> Self
    where
        F: for<'b> FnOnce(MigrationAssertion<'b>) -> AssertionResult<MigrationAssertion<'b>>,
    {
        let migration = std::fs::read_dir(self.migrations_directory.path())
            .context("Reading migrations directory for named migration.")
            .unwrap()
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

                assertions(assertion).unwrap();
            }
            None => panic!(
                "Assertion error. Could not find migration with name matching `{}`",
                name_matcher
            ),
        }

        self
    }

    pub fn output(&self) -> &CreateMigrationOutput {
        &self.output
    }

    #[track_caller]
    pub fn modify_migration<F>(self, modify: F) -> Self
    where
        F: FnOnce(&mut String),
    {
        use std::io::Write as _;

        let migration_script_path = self
            .migrations_directory
            .path()
            .join(self.output.generated_migration_name.as_ref().unwrap())
            .join("migration.sql");

        let new_contents = {
            let mut contents = std::fs::read_to_string(&migration_script_path)
                .context("Reading migration script")
                .unwrap();

            modify(&mut contents);

            contents
        };

        let mut file = std::fs::File::create(&migration_script_path).unwrap();
        write!(file, "{}", new_contents).unwrap();

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
    pub fn assert_contents(self, expected_contents: &str) -> AssertionResult<Self> {
        let migration_file_path = self.path.join("migration.sql");
        let contents: String = std::fs::read_to_string(&migration_file_path)
            .with_context(|| format!("Trying to read migration file at {:?}", migration_file_path))?;

        assert_eq!(expected_contents, contents);

        Ok(self)
    }
}
