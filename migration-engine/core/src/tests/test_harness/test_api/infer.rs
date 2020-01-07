use super::{TestApi, MIGRATION_ID_COUNTER};
use crate::{
    api::GenericApi,
    commands::{InferMigrationStepsInput, MigrationStepsResultOutput},
};
use migration_connector::MigrationStep;

pub struct Infer<'a> {
    pub(super) api: &'a TestApi,
    pub(super) assume_to_be_applied: Option<Vec<MigrationStep>>,
    pub(super) datamodel: String,
    pub(super) migration_id: Option<String>,
}

impl Infer<'_> {
    pub fn migration_id(mut self, migration_id: Option<String>) -> Self {
        self.migration_id = migration_id;
        self
    }

    pub fn assume_to_be_applied(mut self, assume_to_be_applied: Option<Vec<MigrationStep>>) -> Self {
        self.assume_to_be_applied = assume_to_be_applied;
        self
    }

    pub async fn send(self) -> Result<MigrationStepsResultOutput, anyhow::Error> {
        let migration_id = self.migration_id.unwrap_or_else(|| {
            format!(
                "migration-{}",
                MIGRATION_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            )
        });

        let input = InferMigrationStepsInput {
            assume_to_be_applied: self.assume_to_be_applied.unwrap_or_else(Vec::new),
            datamodel: self.datamodel,
            migration_id,
        };

        Ok(self.api.api.infer_migration_steps(&input).await?)
    }
}
