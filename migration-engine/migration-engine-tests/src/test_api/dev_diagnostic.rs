use migration_core::{commands::DevDiagnosticInput, commands::DevDiagnosticOutput, CoreError, CoreResult, GenericApi};
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See DevDiagnostic::send()"]
pub struct DevDiagnostic<'a> {
    api: &'a dyn GenericApi,
    migrations_directory: &'a TempDir,
    rt: &'a tokio::runtime::Runtime,
}

impl<'a> DevDiagnostic<'a> {
    pub(crate) fn new(
        api: &'a dyn GenericApi,
        migrations_directory: &'a TempDir,
        rt: &'a tokio::runtime::Runtime,
    ) -> Self {
        DevDiagnostic {
            api,
            migrations_directory,
            rt,
        }
    }

    fn send_impl(self) -> CoreResult<DevDiagnosticAssertions<'a>> {
        let output = self.rt.block_on(self.api.dev_diagnostic(&DevDiagnosticInput {
            migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
        }))?;

        Ok(DevDiagnosticAssertions {
            output,
            _api: self.api,
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
    _api: &'a dyn GenericApi,
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
