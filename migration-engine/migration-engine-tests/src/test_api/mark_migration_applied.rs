use migration_core::{
    commands::MarkMigrationAppliedInput, commands::MarkMigrationAppliedOutput, CoreError, CoreResult, GenericApi,
};
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See MarkMigrationApplied::send()"]
pub struct MarkMigrationApplied<'a> {
    api: &'a dyn GenericApi,
    migrations_directory: &'a TempDir,
    migration_name: String,
    rt: &'a tokio::runtime::Runtime,
}

impl<'a> MarkMigrationApplied<'a> {
    pub(crate) fn new(
        api: &'a dyn GenericApi,
        migration_name: String,
        migrations_directory: &'a TempDir,
        rt: &'a tokio::runtime::Runtime,
    ) -> Self {
        MarkMigrationApplied {
            api,
            migrations_directory,
            migration_name,
            rt,
        }
    }

    pub fn send_impl(self) -> CoreResult<MarkMigrationAppliedAssertion<'a>> {
        let output = self
            .rt
            .block_on(self.api.mark_migration_applied(&MarkMigrationAppliedInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
                migration_name: self.migration_name,
            }))?;

        Ok(MarkMigrationAppliedAssertion {
            _output: output,
            _api: self.api,
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
    _api: &'a dyn GenericApi,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for MarkMigrationAppliedAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MarkMigrationAppliedAssertion {{ .. }}")
    }
}
