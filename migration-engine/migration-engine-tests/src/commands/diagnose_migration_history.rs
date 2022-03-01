use migration_core::{
    commands::DiagnoseMigrationHistoryOutput,
    commands::{diagnose_migration_history, DiagnoseMigrationHistoryInput},
    migration_connector::MigrationConnector,
    CoreError, CoreResult,
};
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See DiagnoseMigrationHistory::send()"]
pub struct DiagnoseMigrationHistory<'a> {
    api: &'a mut dyn MigrationConnector,
    migrations_directory: &'a TempDir,
    opt_in_to_shadow_database: bool,
}

impl<'a> DiagnoseMigrationHistory<'a> {
    pub fn new(api: &'a mut dyn MigrationConnector, migrations_directory: &'a TempDir) -> Self {
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
        let output = diagnose_migration_history(
            DiagnoseMigrationHistoryInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
                opt_in_to_shadow_database: self.opt_in_to_shadow_database,
            },
            self.api,
        )
        .await?;

        Ok(DiagnoseMigrationHistoryAssertions {
            output,
            _migrations_directory: self.migrations_directory,
        })
    }

    #[track_caller]
    pub fn send_sync(self) -> DiagnoseMigrationHistoryAssertions<'a> {
        test_setup::runtime::run_with_thread_local_runtime(self.send()).unwrap()
    }

    #[track_caller]
    pub fn send_unwrap_err(self) -> CoreError {
        test_setup::runtime::run_with_thread_local_runtime(self.send()).unwrap_err()
    }
}

pub struct DiagnoseMigrationHistoryAssertions<'a> {
    output: DiagnoseMigrationHistoryOutput,
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
