use migration_core::{
    api::GenericApi,
    commands::{UnapplyMigrationInput, UnapplyMigrationOutput},
};

#[derive(Clone)]
pub struct UnapplyMigration<'a> {
    pub(super) api: &'a dyn GenericApi,
    pub(super) force: Option<bool>,
}

impl UnapplyMigration<'_> {
    pub fn force(mut self, force: Option<bool>) -> Self {
        self.force = force;

        self
    }

    pub async fn send(self) -> Result<UnapplyMigrationOutput, anyhow::Error> {
        let input = UnapplyMigrationInput { force: self.force };

        Ok(self.api.unapply_migration(&input).await?)
    }
}
