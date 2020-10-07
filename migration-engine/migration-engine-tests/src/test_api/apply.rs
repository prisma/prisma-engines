use super::super::unique_migration_id;
use migration_connector::MigrationStep;
use migration_core::{
    api::GenericApi,
    commands::{ApplyMigrationInput, MigrationStepsResultOutput},
    CoreResult,
};

#[derive(Clone)]
pub struct Apply<'a> {
    api: &'a dyn GenericApi,
    migration_id: Option<String>,
    steps: Option<Vec<MigrationStep>>,
    force: Option<bool>,
}

impl Apply<'_> {
    pub fn new(api: &dyn GenericApi) -> Apply<'_> {
        Apply {
            api,
            migration_id: None,
            steps: None,
            force: None,
        }
    }

    pub fn migration_id(mut self, migration_id: Option<impl Into<String>>) -> Self {
        self.migration_id = migration_id.map(Into::into);
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
        Ok(self.send_inner().await?)
    }

    pub async fn send_user_facing(self) -> Result<MigrationStepsResultOutput, user_facing_errors::Error> {
        let api = self.api;
        self.send_inner().await.map_err(|err| api.render_error(err))
    }

    async fn send_inner(self) -> CoreResult<MigrationStepsResultOutput> {
        let migration_id = self.migration_id.unwrap_or_else(unique_migration_id);

        let input = ApplyMigrationInput {
            migration_id,
            force: self.force,
            steps: self.steps.unwrap_or_else(Vec::new),
        };

        self.api.apply_migration(&input).await
    }
}
