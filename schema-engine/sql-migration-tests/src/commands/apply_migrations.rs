use crate::utils;
use schema_core::{
    commands::apply_migrations,
    json_rpc::types::*,
    schema_connector::{Namespaces, SchemaConnector},
    CoreError, CoreResult,
};
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See ApplyMigrations::send()"]
pub struct ApplyMigrations<'a> {
    api: &'a mut dyn SchemaConnector,
    migrations_directory: &'a TempDir,
    namespaces: Option<Namespaces>,
}

impl<'a> ApplyMigrations<'a> {
    pub fn new(
        api: &'a mut dyn SchemaConnector,
        migrations_directory: &'a TempDir,
        mut namespaces: Vec<String>,
    ) -> Self {
        let namespaces = Namespaces::from_vec(&mut namespaces);

        ApplyMigrations {
            api,
            migrations_directory,
            namespaces,
        }
    }

    pub async fn send(self) -> CoreResult<ApplyMigrationsAssertion<'a>> {
        let migrations_list = utils::list_migrations(self.migrations_directory.path()).unwrap();
        let output = apply_migrations(ApplyMigrationsInput { migrations_list }, self.api, self.namespaces).await?;

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

impl ApplyMigrationsAssertion<'_> {
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
            "Assertion failed. The applied migrations do not match the expectations. ({found_names:?} vs {names:?})"
        );
        self
    }

    pub fn into_output(self) -> ApplyMigrationsOutput {
        self.output
    }
}
