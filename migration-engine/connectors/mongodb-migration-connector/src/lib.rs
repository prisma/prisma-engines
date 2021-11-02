//! The MongoDB migration connector.
//!
//! It is intentionally structured after sql-migration-connector and implements the same
//! [MigrationConnector](/trait.MigrationConnector.html) API.

mod client_wrapper;
mod destructive_change_checker;
mod differ;
mod migration;
mod migration_persistence;
mod migration_step_applier;
mod schema;
mod schema_calculator;

use client_wrapper::Client;
use migration::MongoDbMigration;
use migration_connector::{ConnectorError, ConnectorResult, DiffTarget, Migration, MigrationConnector};
use schema::MongoSchema;
use tokio::sync::OnceCell;

/// The top-level MongoDB migration connector.
pub struct MongoDbMigrationConnector {
    connection_string: String,
    client: OnceCell<Client>,
}

impl MongoDbMigrationConnector {
    pub fn new(connection_string: String) -> Self {
        Self {
            connection_string,
            client: OnceCell::new(),
        }
    }

    async fn client(&self) -> ConnectorResult<&Client> {
        let client: &Client = self
            .client
            .get_or_try_init(move || Box::pin(async move { Client::connect(&self.connection_string).await }))
            .await?;

        Ok(client)
    }

    /// Only for qe_setup. This should disappear soon.
    pub async fn create_collections(&self, schema: &datamodel::dml::Datamodel) -> ConnectorResult<()> {
        let client = self.client().await?;

        for model in &schema.models {
            client
                .database()
                .create_collection(model.database_name.as_deref().unwrap_or(&model.name), None)
                .await
                .unwrap();
        }

        Ok(())
    }

    async fn mongodb_schema_from_diff_target(&self, target: DiffTarget<'_>) -> ConnectorResult<MongoSchema> {
        match target {
            DiffTarget::Datamodel((_config, schema)) => Ok(schema_calculator::calculate(schema)),
            DiffTarget::Database => self.client().await?.describe().await,
            DiffTarget::Migrations(_) => Err(unsupported_command_error()),
            DiffTarget::Empty => Ok(MongoSchema::default()),
        }
    }
}

#[async_trait::async_trait]
impl MigrationConnector for MongoDbMigrationConnector {
    fn connector_type(&self) -> &'static str {
        "mongodb"
    }

    async fn create_database(&self) -> ConnectorResult<String> {
        Err(ConnectorError::from_msg(
            "create_database() is not supported on mongodb: databases are created automatically when used.".to_owned(),
        ))
    }

    async fn ensure_connection_validity(&self) -> ConnectorResult<()> {
        Ok(())
    }

    async fn version(&self) -> migration_connector::ConnectorResult<String> {
        Ok("4 or 5".to_owned())
    }

    async fn diff(&self, from: DiffTarget<'_>, to: DiffTarget<'_>) -> ConnectorResult<Migration> {
        let from = self.mongodb_schema_from_diff_target(from).await?;
        let to = self.mongodb_schema_from_diff_target(to).await?;
        Ok(Migration::new(differ::diff(from, to)))
    }

    async fn drop_database(&self) -> ConnectorResult<()> {
        self.client().await?.drop_database().await
    }

    fn migration_file_extension(&self) -> &'static str {
        unreachable!("migration_file_extension")
    }

    fn migration_len(&self, migration: &Migration) -> usize {
        migration.downcast_ref::<MongoDbMigration>().steps.len()
    }

    fn migration_summary(&self, migration: &Migration) -> String {
        migration.downcast_ref::<MongoDbMigration>().summary()
    }

    async fn reset(&self) -> migration_connector::ConnectorResult<()> {
        self.client().await?.drop_database().await
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
        Ok(())
    }

    async fn validate_migrations(
        &self,
        _migrations: &[migration_connector::migrations_directory::MigrationDirectory],
    ) -> migration_connector::ConnectorResult<()> {
        Ok(())
    }
}

fn unsupported_command_error() -> ConnectorError {
    ConnectorError::from_msg(
"The \"mongodb\" provider is not supported with this command. For more info see https://www.prisma.io/docs/concepts/database-connectors/mongodb".to_owned()

        )
}
