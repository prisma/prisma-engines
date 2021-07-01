//! The MongoDB migration connector.

mod error;
mod mongodb_destructive_change_checker;
mod mongodb_migration;
mod mongodb_migration_persistence;
mod mongodb_migration_step_applier;

use error::IntoConnectorResult;
use migration_connector::{ConnectorError, ConnectorResult, DiffTarget, Migration, MigrationConnector};
use mongodb::{
    options::{ClientOptions, WriteConcern},
    Client,
};
use mongodb_migration::*;
use url::Url;

/// The top-level MongoDB migration connector.
pub struct MongoDbMigrationConnector {
    client: Client,
    db_name: String,
}

impl MongoDbMigrationConnector {
    /// Construct and initialize the SQL migration connector.
    pub async fn new(database_str: &str) -> ConnectorResult<Self> {
        let (client, db_name) = Self::create_client(database_str).await?;

        Ok(Self { client, db_name })
    }

    /// Set up the database for connector-test-kit, without initializing the connector.
    pub async fn qe_setup(database_str: &str) -> ConnectorResult<()> {
        let (client, db_name) = Self::create_client(database_str).await?;

        // Drop database. Creation is automatically done when collections are created.
        client
            .database(&db_name)
            .drop(Some(
                mongodb::options::DropDatabaseOptions::builder()
                    .write_concern(WriteConcern::builder().journal(true).build())
                    .build(),
            ))
            .await
            .into_connector_result()?;

        Ok(())
    }

    async fn create_client(database_str: &str) -> ConnectorResult<(Client, String)> {
        let url = Url::parse(database_str).map_err(ConnectorError::url_parse_error)?;
        let db_name = url.path().trim_start_matches('/').to_string();

        let client_options = ClientOptions::parse(database_str).await.into_connector_result()?;
        Ok((Client::with_options(client_options).into_connector_result()?, db_name))
    }
}

#[async_trait::async_trait]
impl MigrationConnector for MongoDbMigrationConnector {
    fn connector_type(&self) -> &'static str {
        "mongodb"
    }

    async fn version(&self) -> migration_connector::ConnectorResult<String> {
        Ok("4".to_owned())
    }

    async fn diff(&self, from: DiffTarget<'_>, to: DiffTarget<'_>) -> ConnectorResult<Migration> {
        match (from, to) {
            (DiffTarget::Empty, DiffTarget::Datamodel((_, datamodel))) => {
                let steps = datamodel
                    .models()
                    .map(|model| {
                        let name = model.database_name.as_ref().unwrap_or(&model.name).to_owned();
                        MongoDbMigrationStep::CreateCollection(name)
                    })
                    .collect();

                Ok(Migration::new(MongoDbMigration { steps }))
            }
            _ => todo!(),
        }
    }

    fn migration_file_extension(&self) -> &'static str {
        "mongo"
    }

    fn migration_len(&self, migration: &Migration) -> usize {
        migration.downcast_ref::<MongoDbMigration>().steps.len()
    }

    fn migration_summary(&self, migration: &Migration) -> String {
        migration.downcast_ref::<MongoDbMigration>().summary()
    }

    async fn reset(&self) -> migration_connector::ConnectorResult<()> {
        self.client
            .database(&self.db_name)
            .drop(None)
            .await
            .into_connector_result()
    }

    fn migration_persistence(&self) -> &dyn migration_connector::MigrationPersistence {
        self
    }

    fn database_migration_step_applier(&self) -> &dyn migration_connector::DatabaseMigrationStepApplier {
        self
    }

    fn destructive_change_checker(&self) -> &dyn migration_connector::DestructiveChangeChecker {
        self
    }

    async fn acquire_lock(&self) -> ConnectorResult<()> {
        todo!()
    }

    async fn validate_migrations(
        &self,
        _migrations: &[migration_connector::migrations_directory::MigrationDirectory],
    ) -> migration_connector::ConnectorResult<()> {
        Ok(())
    }
}
