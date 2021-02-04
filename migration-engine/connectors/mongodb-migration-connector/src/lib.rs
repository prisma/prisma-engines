//! The MongoDB migration connector.

mod error;
mod mongodb_destructive_change_checker;
mod mongodb_migration;
mod mongodb_migration_persistence;
mod mongodb_migration_step_applier;
mod mongodb_migration_step_inferrer;

use enumflags2::BitFlags;
use error::IntoConnectorResult;
use migration_connector::{ConnectorError, ConnectorResult, MigrationConnector, MigrationFeature};
use mongodb::{options::ClientOptions, Client};
use mongodb_migration::*;
use url::Url;

/// The top-level MongoDB migration connector.
pub struct MongoDbMigrationConnector {
    client: Client,
    db_name: String,
}

impl MongoDbMigrationConnector {
    /// Construct and initialize the SQL migration connector.
    pub async fn new(database_str: &str, _features: BitFlags<MigrationFeature>) -> ConnectorResult<Self> {
        let url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let db_name = url.path().trim_start_matches("/").to_string();

        let client_options = ClientOptions::parse(database_str).await.into_connector_result()?;
        let client = Client::with_options(client_options).into_connector_result()?;

        Ok(Self { client, db_name })
    }

    /// Set up the database for connector-test-kit, without initializing the connector.
    pub async fn qe_setup(database_str: &str) -> ConnectorResult<()> {
        let url = Url::parse(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let db_name = url.path();

        let client_options = ClientOptions::parse(database_str).await.into_connector_result()?;
        let client = Client::with_options(client_options).into_connector_result()?;

        // Drop database. Creation is automatically done when collections are created.
        client.database(db_name).drop(None).await.into_connector_result()?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl MigrationConnector for MongoDbMigrationConnector {
    type DatabaseMigration = MongoDbMigration;

    fn connector_type(&self) -> &'static str {
        "mongodb"
    }

    async fn version(&self) -> migration_connector::ConnectorResult<String> {
        Ok("4".to_owned())
    }

    async fn create_database(_database_str: &str) -> migration_connector::ConnectorResult<String> {
        todo!()
    }

    async fn reset(&self) -> migration_connector::ConnectorResult<()> {
        todo!()
    }

    fn migration_persistence(&self) -> &dyn migration_connector::MigrationPersistence {
        self
    }

    fn database_migration_inferrer(
        &self,
    ) -> &dyn migration_connector::DatabaseMigrationInferrer<Self::DatabaseMigration> {
        self
    }

    fn database_migration_step_applier(
        &self,
    ) -> &dyn migration_connector::DatabaseMigrationStepApplier<Self::DatabaseMigration> {
        self
    }

    fn destructive_change_checker(
        &self,
    ) -> &dyn migration_connector::DestructiveChangeChecker<Self::DatabaseMigration> {
        self
    }
}
