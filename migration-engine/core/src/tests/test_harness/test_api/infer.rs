use super::super::unique_migration_id;
use crate::{
    api::GenericApi,
    commands::{InferMigrationStepsInput, MigrationStepsResultOutput},
};
use migration_connector::MigrationStep;

pub struct Infer<'a> {
    pub(super) api: &'a dyn GenericApi,
    pub(super) assume_to_be_applied: Option<Vec<MigrationStep>>,
    pub(super) datamodel: String,
    pub(super) migration_id: Option<String>,
}

impl Infer<'_> {
    pub fn migration_id(mut self, migration_id: Option<impl Into<String>>) -> Self {
        self.migration_id = migration_id.map(Into::into);
        self
    }

    pub fn assume_to_be_applied(mut self, assume_to_be_applied: Option<Vec<MigrationStep>>) -> Self {
        self.assume_to_be_applied = assume_to_be_applied;
        self
    }

    pub async fn send(self) -> Result<MigrationStepsResultOutput, anyhow::Error> {
        let migration_id = self.migration_id.unwrap_or_else(unique_migration_id);

        let input = InferMigrationStepsInput {
            assume_to_be_applied: self.assume_to_be_applied.unwrap_or_else(Vec::new),
            datamodel: self.datamodel,
            migration_id,
        };

        let output = self.api.infer_migration_steps(&input).await?;

        assert!(
            output.general_errors.is_empty(),
            format!("InferMigration returned unexpected errors: {:?}", output.general_errors)
        );

        Ok(output)
    }
}
