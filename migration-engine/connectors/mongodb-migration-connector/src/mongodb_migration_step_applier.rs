use crate::IntoConnectorResult;
use migration_connector::{DatabaseMigrationStepApplier, DestructiveChangeDiagnostics, Migration};
use mongodb_migration::MongoDbMigrationStep;

use crate::{
    mongodb_migration::{self, MongoDbMigration},
    MongoDbMigrationConnector,
};

#[async_trait::async_trait]
impl DatabaseMigrationStepApplier for MongoDbMigrationConnector {
    async fn apply_migration(&self, migration: &Migration) -> migration_connector::ConnectorResult<u32> {
        let db = self.client.database(&self.db_name);
        let migration: &MongoDbMigration = migration.downcast_ref();

        for step in migration.steps.iter() {
            match step {
                MongoDbMigrationStep::CreateCollection(name) => db
                    .create_collection(name.as_str(), None)
                    .await
                    .into_connector_result()?,
            }
        }

        Ok(migration.steps.len() as u32)
    }

    fn render_script(&self, _migration: &Migration, _diagnostics: &DestructiveChangeDiagnostics) -> String {
        todo!()
    }

    async fn apply_script(&self, _migration_name: &str, _script: &str) -> migration_connector::ConnectorResult<()> {
        todo!()
    }
}
