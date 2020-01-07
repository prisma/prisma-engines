use super::{TestApi, MIGRATION_ID_COUNTER};
use crate::{
    api::GenericApi,
    commands::{ApplyMigrationInput, MigrationStepsResultOutput},
};
use migration_connector::MigrationStep;

#[derive(Clone)]
pub struct Apply<'a> {
    pub(super) api: &'a TestApi,
    pub(super) migration_id: Option<String>,
    pub(super) steps: Option<Vec<MigrationStep>>,
    pub(super) force: Option<bool>,
}

impl Apply<'_> {
    pub fn migration_id(mut self, migration_id: Option<String>) -> Self {
        self.migration_id = migration_id;
        self
    }

    pub fn steps(mut self, steps: Option<Vec<MigrationStep>>) -> Self {
        self.steps = steps;
        self
    }

    pub fn force(mut self, force: Option<bool>) -> Self {
        self.force = force;
        self
    }

    pub async fn send(self) -> Result<MigrationStepsResultOutput, anyhow::Error> {
        let migration_id = self.migration_id.unwrap_or_else(|| {
            format!(
                "migration-{}",
                MIGRATION_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            )
        });

        let input = ApplyMigrationInput {
            migration_id,
            force: self.force,
            steps: self.steps.unwrap_or_else(Vec::new),
        };

        Ok(self.api.api.apply_migration(&input).await?)
    }
}
