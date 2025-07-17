use schema_core::{
    CoreError, CoreResult, commands::mark_migration_rolled_back, json_rpc::types::*, schema_connector::SchemaConnector,
};

#[must_use = "This struct does nothing on its own. See MarkMigrationRolledBack::send()"]
pub struct MarkMigrationRolledBack<'a> {
    api: &'a mut dyn SchemaConnector,
    migration_name: String,
}

impl<'a> MarkMigrationRolledBack<'a> {
    pub fn new(api: &'a mut dyn SchemaConnector, migration_name: String) -> Self {
        MarkMigrationRolledBack { api, migration_name }
    }

    fn send_impl(self) -> CoreResult<MarkMigrationRolledBackAssertion> {
        let output = test_setup::runtime::run_with_thread_local_runtime(mark_migration_rolled_back(
            MarkMigrationRolledBackInput {
                migration_name: self.migration_name,
            },
            self.api,
        ))?;

        Ok(MarkMigrationRolledBackAssertion { _output: output })
    }

    pub fn send(self) -> MarkMigrationRolledBackAssertion {
        self.send_impl().unwrap()
    }

    pub fn send_unwrap_err(self) -> CoreError {
        self.send_impl().unwrap_err()
    }
}

pub struct MarkMigrationRolledBackAssertion {
    _output: MarkMigrationRolledBackOutput,
}

impl std::fmt::Debug for MarkMigrationRolledBackAssertion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MarkMigrationRolledBackAssertion {{ .. }}")
    }
}
