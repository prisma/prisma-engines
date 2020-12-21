use migration_core::{commands::DevDiagnosticInput, commands::DevDiagnosticOutput, CoreResult, GenericApi};
use tempfile::TempDir;

#[must_use = "This struct does nothing on its own. See DevDiagnostic::send()"]
pub struct DevDiagnostic<'a> {
    api: &'a dyn GenericApi,
    migrations_directory: &'a TempDir,
}

impl<'a> DevDiagnostic<'a> {
    pub fn new(api: &'a dyn GenericApi, migrations_directory: &'a TempDir) -> Self {
        DevDiagnostic {
            api,
            migrations_directory,
        }
    }

    pub async fn send(self) -> CoreResult<DevDiagnosticAssertions<'a>> {
        let output = self
            .api
            .dev_diagnostic(&DevDiagnosticInput {
                migrations_directory_path: self.migrations_directory.path().to_str().unwrap().to_owned(),
            })
            .await?;

        Ok(DevDiagnosticAssertions {
            output,
            _api: self.api,
            _migrations_directory: self.migrations_directory,
        })
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
