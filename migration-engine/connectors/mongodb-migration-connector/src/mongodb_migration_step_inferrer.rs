use datamodel::{Configuration, Datamodel};
use migration_connector::DatabaseMigrationInferrer;

use crate::{
    mongodb_migration::{MongoDbMigration, MongoDbMigrationStep},
    MongoDbMigrationConnector,
};

#[async_trait::async_trait]
impl DatabaseMigrationInferrer<MongoDbMigration> for MongoDbMigrationConnector {
    async fn infer(
        &self,
        next: (&Configuration, &Datamodel),
    ) -> migration_connector::ConnectorResult<MongoDbMigration> {
        self.infer_from_empty(next)
    }

    fn infer_from_empty(
        &self,
        next: (&Configuration, &Datamodel),
    ) -> migration_connector::ConnectorResult<MongoDbMigration> {
        let steps = next
            .1
            .models()
            .map(|model| {
                let name = model.database_name.as_ref().unwrap_or(&model.name).to_owned();
                MongoDbMigrationStep::CreateCollection(name)
            })
            .collect();

        Ok(MongoDbMigration { steps })
    }

    async fn infer_next_migration(
        &self,
        _previous_migrations: &[migration_connector::MigrationDirectory],
        _target_schema: (&Configuration, &Datamodel),
    ) -> migration_connector::ConnectorResult<MongoDbMigration> {
        todo!()
    }

    async fn calculate_drift(
        &self,
        _applied_migrations: &[migration_connector::MigrationDirectory],
    ) -> migration_connector::ConnectorResult<Option<String>> {
        Ok(None)
    }

    async fn validate_migrations(
        &self,
        _migrations: &[migration_connector::MigrationDirectory],
    ) -> migration_connector::ConnectorResult<()> {
        Ok(())
    }
}
