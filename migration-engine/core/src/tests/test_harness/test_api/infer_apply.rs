use super::super::unique_migration_id;
use crate::{
    api::GenericApi,
    commands::{ApplyMigrationInput, InferMigrationStepsInput, MigrationStepsResultOutput},
};

pub struct InferApply<'a> {
    pub(super) api: &'a dyn GenericApi,
    pub(super) schema: &'a str,
    pub(super) migration_id: Option<String>,
    pub(super) force: Option<bool>,
}

impl<'a> InferApply<'a> {
    pub fn force(mut self, force: Option<bool>) -> Self {
        self.force = force;
        self
    }

    pub fn migration_id(mut self, migration_id: Option<impl Into<String>>) -> Self {
        self.migration_id = migration_id.map(Into::into);
        self
    }

    pub async fn send(self) -> Result<MigrationStepsResultOutput, anyhow::Error> {
        let migration_id = self.migration_id.map(Into::into).unwrap_or_else(unique_migration_id);

        let input = InferMigrationStepsInput {
            migration_id: migration_id.clone(),
            datamodel: self.schema.to_owned(),
            assume_to_be_applied: Some(Vec::new()),
            assume_applied_migrations: None,
        };

        let steps = self.api.infer_migration_steps(&input).await?.datamodel_steps;

        let input = ApplyMigrationInput {
            migration_id,
            steps,
            force: self.force,
        };

        let migration_output = self.api.apply_migration(&input).await?;

        Ok(migration_output)
    }
}
