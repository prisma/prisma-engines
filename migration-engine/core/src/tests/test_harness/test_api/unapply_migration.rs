use crate::{
    api::GenericApi,
    commands::{UnapplyMigrationInput, UnapplyMigrationOutput},
};

#[derive(Clone)]
pub(crate) struct UnapplyMigration<'a> {
    pub(super) api: &'a dyn GenericApi,
}

impl UnapplyMigration<'_> {
    pub(crate) async fn send(self) -> Result<UnapplyMigrationOutput, anyhow::Error> {
        let input = UnapplyMigrationInput {};

        Ok(self.api.unapply_migration(&input).await?)
    }
}
