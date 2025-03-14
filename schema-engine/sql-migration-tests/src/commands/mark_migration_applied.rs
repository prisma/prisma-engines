use schema_core::{
    commands::mark_migration_applied, json_rpc::types::*, schema_connector::SchemaConnector, CoreError, CoreResult,
};
use tempfile::TempDir;

use crate::utils;

#[must_use = "This struct does nothing on its own. See MarkMigrationApplied::send()"]
pub struct MarkMigrationApplied<'a> {
    api: &'a mut dyn SchemaConnector,
    migrations_directory: &'a TempDir,
    migration_name: String,
}

impl<'a> MarkMigrationApplied<'a> {
    pub(crate) fn new(
        api: &'a mut dyn SchemaConnector,
        migration_name: String,
        migrations_directory: &'a TempDir,
    ) -> Self {
        MarkMigrationApplied {
            api,
            migrations_directory,
            migration_name,
        }
    }

    pub fn send_impl(self) -> CoreResult<MarkMigrationAppliedAssertion<'a>> {
        let migrations_list = utils::list_migrations(self.migrations_directory.path()).unwrap();
        let output = test_setup::runtime::run_with_thread_local_runtime(mark_migration_applied(
            MarkMigrationAppliedInput {
                migrations_list,
                migration_name: self.migration_name,
            },
            self.api,
        ))?;
        Ok(MarkMigrationAppliedAssertion {
            _output: output,
            _migrations_directory: self.migrations_directory,
        })
    }

    pub fn send(self) -> MarkMigrationAppliedAssertion<'a> {
        self.send_impl().unwrap()
    }

    pub fn send_unwrap_err(self) -> CoreError {
        self.send_impl().unwrap_err()
    }
}

pub struct MarkMigrationAppliedAssertion<'a> {
    _output: MarkMigrationAppliedOutput,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for MarkMigrationAppliedAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MarkMigrationAppliedAssertion {{ .. }}")
    }
}
