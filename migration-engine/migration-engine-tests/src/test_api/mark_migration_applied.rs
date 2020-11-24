use migration_core::{
    commands::MarkMigrationAppliedInput, commands::MarkMigrationAppliedOutput, CoreResult, GenericApi,
};
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See MarkMigrationApplied::send()"]
pub struct MarkMigrationApplied<'a> {
    api: &'a dyn GenericApi,
    migrations_directory: &'a TempDir,
    migration_name: String,
}

impl<'a> MarkMigrationApplied<'a> {
    pub fn new(api: &'a dyn GenericApi, migration_name: String, migrations_directory: &'a TempDir) -> Self {
        MarkMigrationApplied {
            api,
            migrations_directory,
            migration_name,
        }
    }

    pub async fn send(self) -> CoreResult<MarkMigrationAppliedAssertion<'a>> {
        let output = self
            .api
            .mark_migration_applied(&MarkMigrationAppliedInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
                migration_name: self.migration_name,
            })
            .await?;

        Ok(MarkMigrationAppliedAssertion {
            _output: output,
            _api: self.api,
            _migrations_directory: self.migrations_directory,
        })
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
