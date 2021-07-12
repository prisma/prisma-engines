use migration_core::{
    commands::MarkMigrationRolledBackInput, commands::MarkMigrationRolledBackOutput, CoreError, CoreResult, GenericApi,
};

#[must_use = "This struct does nothing on its own. See MarkMigrationRolledBack::send()"]
pub struct MarkMigrationRolledBack<'a> {
    api: &'a dyn GenericApi,
    migration_name: String,
    rt: &'a tokio::runtime::Runtime,
}

impl<'a> MarkMigrationRolledBack<'a> {
    pub fn new(api: &'a dyn GenericApi, migration_name: String, rt: &'a tokio::runtime::Runtime) -> Self {
        MarkMigrationRolledBack {
            api,
            migration_name,
            rt,
        }
    }

    fn send_impl(self) -> CoreResult<MarkMigrationRolledBackAssertion<'a>> {
        let output = self
            .rt
            .block_on(self.api.mark_migration_rolled_back(&MarkMigrationRolledBackInput {
                migration_name: self.migration_name,
            }))?;

        Ok(MarkMigrationRolledBackAssertion {
            _output: output,
            _api: self.api,
        })
    }

    pub fn send(self) -> MarkMigrationRolledBackAssertion<'a> {
        self.send_impl().unwrap()
    }

    pub fn send_unwrap_err(self) -> CoreError {
        self.send_impl().unwrap_err()
    }
}

pub struct MarkMigrationRolledBackAssertion<'a> {
    _output: MarkMigrationRolledBackOutput,
    _api: &'a dyn GenericApi,
}

impl std::fmt::Debug for MarkMigrationRolledBackAssertion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MarkMigrationRolledBackAssertion {{ .. }}")
    }
}
