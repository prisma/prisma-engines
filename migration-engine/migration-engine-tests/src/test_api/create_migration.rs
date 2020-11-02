use anyhow::Context;
use migration_core::{commands::CreateMigrationInput, commands::CreateMigrationOutput, GenericApi};
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
}

impl<'a> CreateMigration<'a> {
    pub fn new(api: &'a dyn GenericApi, name: &'a str, schema: &'a str, migrations_directory: &'a TempDir) -> Self {
        CreateMigration {
            api,
            schema,
            migrations_directory,
            draft: false,
            name,
        }
    }

    pub fn draft(mut self, draft: bool) -> Self {
        self.draft = draft;

        self
    }

    pub async fn send(self) -> anyhow::Result<CreateMigrationAssertion<'a>> {
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
    pub fn assert_migration_directories_count(self, expected_count: usize) -> AssertionResult<Self> {
        let mut count = 0;

        for _ in std::fs::read_dir(self.migrations_directory.path())
            .context("Counting directories in migrations directory.")?
        {
            count += 1;
        }

        anyhow::ensure!(
            expected_count == count,
            "Assertion failed. Expected {expected} migrations in the migrations directory, found {actual}.",
            expected = expected_count,
            actual = count
        );

        Ok(self)
    }

    /// Assert that there is one migration with `name_matcher` contained in its name present in the migration directory.
    pub fn assert_migration<F>(self, name_matcher: &str, assertions: F) -> AssertionResult<Self>
    where
        F: for<'b> FnOnce(MigrationAssertion<'b>) -> AssertionResult<MigrationAssertion<'b>>,
    {
        let migration = std::fs::read_dir(self.migrations_directory.path())
            .context("Reading migrations directory for named migration.")?
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

                assertions(assertion)?;
            }
            None => anyhow::bail!(
                "Assertion error. Could not find migration with name matching `{}`",
                name_matcher
            ),
        }

        Ok(self)
    }

    pub fn output(&self) -> &CreateMigrationOutput {
        &self.output
    }

    pub fn modify_migration<F>(self, modify: F) -> AssertionResult<Self>
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
            let mut contents = std::fs::read_to_string(&migration_script_path).context("Reading migration script")?;

            modify(&mut contents);

            contents
        };

        let mut file = std::fs::File::create(&migration_script_path)?;
        write!(file, "{}", new_contents)?;

        Ok(self)
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
