use migration_core::{
    commands::DiagnoseMigrationHistoryInput, commands::DiagnoseMigrationHistoryOutput, CoreResult, GenericApi,
};
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See DiagnoseMigrationHistory::send()"]
pub struct DiagnoseMigrationHistory<'a> {
    api: &'a dyn GenericApi,
    migrations_directory: &'a TempDir,
    opt_in_to_shadow_database: bool,
}

impl<'a> DiagnoseMigrationHistory<'a> {
    pub fn new(api: &'a dyn GenericApi, migrations_directory: &'a TempDir) -> Self {
        DiagnoseMigrationHistory {
            api,
            migrations_directory,
            opt_in_to_shadow_database: false,
        }
    }

    pub fn opt_in_to_shadow_database(mut self, opt_in_to_shadow_database: bool) -> Self {
        self.opt_in_to_shadow_database = opt_in_to_shadow_database;

        self
    }

    pub async fn send(self) -> CoreResult<DiagnoseMigrationHistoryAssertions<'a>> {
        let output = self
            .api
            .diagnose_migration_history(&DiagnoseMigrationHistoryInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
                opt_in_to_shadow_database: self.opt_in_to_shadow_database,
            })
            .await?;

        Ok(DiagnoseMigrationHistoryAssertions {
            output,
            _api: self.api,
            _migrations_directory: self.migrations_directory,
        })
    }
}

pub struct DiagnoseMigrationHistoryAssertions<'a> {
    output: DiagnoseMigrationHistoryOutput,
    _api: &'a dyn GenericApi,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for DiagnoseMigrationHistoryAssertions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DiagnoseMigrationHistoryAssertions {{ .. }}")
    }
}

impl<'a> DiagnoseMigrationHistoryAssertions<'a> {
    pub fn into_output(self) -> DiagnoseMigrationHistoryOutput {
        self.output
    }
}
