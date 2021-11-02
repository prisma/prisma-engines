use crate::{
    client_wrapper::mongo_error_to_connector_error,
    migration::{MongoDbMigration, MongoDbMigrationStep},
    MongoDbMigrationConnector,
};
use migration_connector::{
    ConnectorResult, DatabaseMigrationStepApplier, DestructiveChangeDiagnostics, Migration, MigrationConnector,
};

#[async_trait::async_trait]
impl DatabaseMigrationStepApplier for MongoDbMigrationConnector {
    async fn apply_migration(&self, migration: &Migration) -> ConnectorResult<u32> {
        let db = self.client().await?.database();

        if !self.migration_is_empty(migration) {
            tracing::info!(
                migrate_action = "log",
                "Applying the following changes:\n\n{}",
                self.migration_summary(migration)
            );
        }

        let migration: &MongoDbMigration = migration.downcast_ref();

        for step in migration.steps.iter() {
            match step {
                MongoDbMigrationStep::CreateCollection(id) => {
                    db.create_collection(migration.next.walk_collection(*id).name(), None)
                        .await
                        .map_err(mongo_error_to_connector_error)?;
                }
                MongoDbMigrationStep::CreateIndex(index_id) => {
                    let index = migration.next.walk_index(*index_id);
                    let collection: mongodb::Collection<bson::Document> = db.collection(index.collection().name());

                    let mut index_model = mongodb::IndexModel::default();
                    index_model.keys = index.keys().clone();
                    let mut index_options = mongodb::options::IndexOptions::default();
                    index_options.name = Some(index.name().to_owned());
                    index_options.unique = Some(index.is_unique());
                    index_model.options = Some(index_options);
                    collection
                        .create_index(index_model, None)
                        .await
                        .map_err(mongo_error_to_connector_error)?;
                }
                MongoDbMigrationStep::DropIndex(index_id) => {
                    let index = migration.previous.walk_index(*index_id);
                    let collection: mongodb::Collection<bson::Document> = db.collection(index.collection().name());
                    collection
                        .drop_index(index.name(), None)
                        .await
                        .map_err(mongo_error_to_connector_error)?;
                }
            }
        }

        Ok(migration.steps.len() as u32)
    }

    fn render_script(&self, _migration: &Migration, _diagnostics: &DestructiveChangeDiagnostics) -> String {
        unreachable!()
    }

    async fn apply_script(&self, _migration_name: &str, _script: &str) -> ConnectorResult<()> {
        Err(crate::unsupported_command_error())
    }
}
