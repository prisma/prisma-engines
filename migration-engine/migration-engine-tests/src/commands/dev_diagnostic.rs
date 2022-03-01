use migration_core::{
    commands::dev_diagnostic, json_rpc::types::*, migration_connector::MigrationConnector, CoreError, CoreResult,
};
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See DevDiagnostic::send()"]
pub struct DevDiagnostic<'a> {
    api: &'a mut dyn MigrationConnector,
    migrations_directory: &'a TempDir,
}

impl<'a> DevDiagnostic<'a> {
    pub(crate) fn new(api: &'a mut dyn MigrationConnector, migrations_directory: &'a TempDir) -> Self {
        DevDiagnostic {
            api,
            migrations_directory,
        }
    }

    fn send_impl(self) -> CoreResult<DevDiagnosticAssertions<'a>> {
        let fut = dev_diagnostic(
            DevDiagnosticInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
            },
            self.api,
        );
        let output = test_setup::runtime::run_with_thread_local_runtime(fut)?;
        Ok(DevDiagnosticAssertions {
            output,
            _migrations_directory: self.migrations_directory,
        })
    }

    #[track_caller]
    pub fn send(self) -> DevDiagnosticAssertions<'a> {
        self.send_impl().unwrap()
    }

    #[track_caller]
    pub fn send_unwrap_err(self) -> CoreError {
        self.send_impl().unwrap_err()
    }
}

pub struct DevDiagnosticAssertions<'a> {
    output: DevDiagnosticOutput,
    _migrations_directory: &'a TempDir,
}

impl std::fmt::Debug for DevDiagnosticAssertions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DevDiagnosticAssertions {{ .. }}")
    }
}

impl<'a> DevDiagnosticAssertions<'a> {
    pub fn into_output(self) -> DevDiagnosticOutput {
        self.output
    }
}
