use super::super::unique_migration_id;
use migration_connector::MigrationStep;
use migration_core::{
    api::GenericApi,
    commands::{AppliedMigration, InferMigrationStepsInput, MigrationStepsResultOutput},
};

pub struct Infer<'a> {
    pub(super) api: &'a dyn GenericApi,
    pub(super) assume_to_be_applied: Option<Vec<MigrationStep>>,
    pub(super) assume_applied_migrations: Option<Vec<AppliedMigration>>,
    pub(super) datamodel: String,
    pub(super) migration_id: Option<String>,
}

impl<'a> Infer<'a> {
    pub fn new(api: &'a dyn GenericApi, dm: impl Into<String>) -> Self {
        Infer {
            api,
            datamodel: dm.into(),
            assume_to_be_applied: None,
            assume_applied_migrations: None,
            migration_id: None,
        }
    }

    pub fn migration_id(mut self, migration_id: Option<impl Into<String>>) -> Self {
        self.migration_id = migration_id.map(Into::into);
        self
    }

    pub fn assume_to_be_applied(mut self, assume_to_be_applied: Option<Vec<MigrationStep>>) -> Self {
        self.assume_to_be_applied = assume_to_be_applied;
        self
    }

    pub fn assume_applied_migrations(mut self, assume_applied_migrations: Option<Vec<AppliedMigration>>) -> Self {
        self.assume_applied_migrations = assume_applied_migrations;
        self
    }

    pub async fn send(self) -> Result<MigrationStepsResultOutput, anyhow::Error> {
        let migration_id = self.migration_id.unwrap_or_else(unique_migration_id);

        let input = InferMigrationStepsInput {
            assume_to_be_applied: Some(self.assume_to_be_applied.unwrap_or_else(Vec::new)),
            assume_applied_migrations: self.assume_applied_migrations,
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
