use migration_core::{
    commands::MarkMigrationRolledBackInput, commands::MarkMigrationRolledBackOutput, CoreResult, GenericApi,
};

#[must_use = "This struct does nothing on its own. See MarkMigrationRolledBack::send()"]
pub struct MarkMigrationRolledBack<'a> {
    api: &'a dyn GenericApi,
    migration_name: String,
}

impl<'a> MarkMigrationRolledBack<'a> {
    pub fn new(api: &'a dyn GenericApi, migration_name: String) -> Self {
        MarkMigrationRolledBack { api, migration_name }
    }

    pub async fn send(self) -> CoreResult<MarkMigrationRolledBackAssertion<'a>> {
        let output = self
            .api
            .mark_migration_rolled_back(&MarkMigrationRolledBackInput {
                migration_name: self.migration_name,
            })
            .await?;

        Ok(MarkMigrationRolledBackAssertion {
            _output: output,
            _api: self.api,
        })
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
