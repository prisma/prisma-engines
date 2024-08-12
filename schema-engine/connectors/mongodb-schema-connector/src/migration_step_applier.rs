use crate::{
    client_wrapper::mongo_error_to_connector_error,
    migration::{MongoDbMigration, MongoDbMigrationStep},
    MongoDbSchemaConnector,
};
use mongodb::bson::{self, Bson, Document};
use schema_connector::{ConnectorResult, Migration, SchemaConnector};

impl MongoDbSchemaConnector {
    pub(crate) async fn apply_migration_impl(&self, migration: &Migration) -> ConnectorResult<u32> {
        let db = self.client().await?.database();

        if !self.migration_is_empty(migration) {
            self.host
                .print(&format!(
                    "Applying the following changes:\n\n{}\n",
                    self.migration_summary(migration)
                ))
                .await?;
        }

        let migration: &MongoDbMigration = migration.downcast_ref();

        for step in migration.steps.iter() {
            match step {
                MongoDbMigrationStep::CreateCollection(id) => {
                    db.create_collection(migration.next.walk_collection(*id).name())
                        .await
                        .map_err(mongo_error_to_connector_error)?;
                }
                MongoDbMigrationStep::CreateIndex(index_id) => {
                    let index = migration.next.walk_index(*index_id);
                    let collection: mongodb::Collection<bson::Document> = db.collection(index.collection().name());

                    let mut index_model = mongodb::IndexModel::default();

                    index_model.keys = index.fields().fold(Document::new(), |mut acc, field| {
                        acc.insert(field.name().to_string(), Bson::from(field.property));
                        acc
                    });

                    let mut index_options = mongodb::options::IndexOptions::default();
                    index_options.name = Some(index.name().to_owned());
                    index_options.unique = Some(index.is_unique());
                    index_model.options = Some(index_options);
                    collection
                        .create_index(index_model)
                        .await
                        .map_err(mongo_error_to_connector_error)?;
                }
                MongoDbMigrationStep::DropIndex(index_id) => {
                    let index = migration.previous.walk_index(*index_id);
                    let collection: mongodb::Collection<bson::Document> = db.collection(index.collection().name());
                    collection
                        .drop_index(index.name())
                        .await
                        .map_err(mongo_error_to_connector_error)?;
                }
            }
        }

        Ok(migration.steps.len() as u32)
    }
}
