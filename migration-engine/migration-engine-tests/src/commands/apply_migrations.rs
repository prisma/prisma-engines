use migration_core::{
    commands::apply_migrations, json_rpc::types::*, migration_connector::MigrationConnector, CoreError, CoreResult,
};
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See ApplyMigrations::send()"]
pub struct ApplyMigrations<'a> {
    api: &'a mut dyn MigrationConnector,
    migrations_directory: &'a TempDir,
}

impl<'a> ApplyMigrations<'a> {
    pub fn new(api: &'a mut dyn MigrationConnector, migrations_directory: &'a TempDir) -> Self {
        ApplyMigrations {
            api,
            migrations_directory,
        }
    }

    pub async fn send(self) -> CoreResult<ApplyMigrationsAssertion<'a>> {
        let output = apply_migrations(
            ApplyMigrationsInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
            },
            self.api,
        )
        .await?;

        Ok(ApplyMigrationsAssertion {
            output,
            _migrations_directory: self.migrations_directory,
        })
    }

    #[track_caller]
    pub fn send_sync(self) -> ApplyMigrationsAssertion<'a> {
        test_setup::runtime::run_with_thread_local_runtime(self.send()).unwrap()
    }

    #[track_caller]
    pub fn send_unwrap_err(self) -> CoreError {
        test_setup::runtime::run_with_thread_local_runtime(self.send()).unwrap_err()
    }
}

pub struct ApplyMigrationsAssertion<'a> {
    output: ApplyMigrationsOutput,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for ApplyMigrationsAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApplyMigrationsAssertion {{ .. }}")
    }
}

impl<'a> ApplyMigrationsAssertion<'a> {
    #[track_caller]
    pub fn assert_applied_migrations(self, names: &[&str]) -> Self {
        let found_names: Vec<&str> = self
            .output
            .applied_migration_names
            .iter()
            .map(|name| &name[15..])
            .collect();

        assert!(
            found_names == names,
            "Assertion failed. The applied migrations do not match the expectations. ({:?} vs {:?})",
            found_names,
            names
        );
        self
    }

    pub fn into_output(self) -> ApplyMigrationsOutput {
        self.output
    }
}
