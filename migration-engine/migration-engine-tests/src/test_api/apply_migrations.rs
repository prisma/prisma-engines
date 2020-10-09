use migration_core::{commands::ApplyMigrationsInput, commands::ApplyMigrationsOutput, GenericApi};
use tempfile::TempDir;

use crate::AssertionResult;

#[must_use = "This struct does nothing on its own. See ApplyMigrations::send()"]
pub struct ApplyMigrations<'a> {
    api: &'a dyn GenericApi,
    migrations_directory: &'a TempDir,
}

impl<'a> ApplyMigrations<'a> {
    pub fn new(api: &'a dyn GenericApi, migrations_directory: &'a TempDir) -> Self {
        ApplyMigrations {
            api,
            migrations_directory,
        }
    }

    pub async fn send(self) -> anyhow::Result<ApplyMigrationsAssertion<'a>> {
        let output = self
            .api
            .apply_migrations(&ApplyMigrationsInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
            })
            .await?;

        Ok(ApplyMigrationsAssertion {
            output,
            _api: self.api,
            _migrations_directory: self.migrations_directory,
        })
    }
}

pub struct ApplyMigrationsAssertion<'a> {
    output: ApplyMigrationsOutput,
    _api: &'a dyn GenericApi,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for ApplyMigrationsAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApplyMigrationsAssertion {{ .. }}")
    }
}

impl<'a> ApplyMigrationsAssertion<'a> {
    pub fn assert_applied_migrations(self, names: &[&str]) -> AssertionResult<Self> {
        let found_names: Vec<&str> = self
            .output
            .applied_migration_names
            .iter()
            .map(|name| &name[15..])
            .collect();

        anyhow::ensure!(
            found_names == names,
            "Assertion failed. The applied migrations do not match the expectations. ({:?} vs {:?})",
            found_names,
            names
        );

        Ok(self)
    }
}
