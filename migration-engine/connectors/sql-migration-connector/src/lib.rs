//! The SQL migration connector.

#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::trivial_regex)] // these will grow

mod connection_wrapper;
mod error;
mod flavour;
mod pair;
mod sql_database_migration_inferrer;
mod sql_database_step_applier;
mod sql_destructive_change_checker;
mod sql_imperative_migration_persistence;
mod sql_migration;
mod sql_migration_persistence;
mod sql_renderer;
mod sql_schema_calculator;
mod sql_schema_differ;

// These will become private.
pub use sql_migration::SqlMigration;
pub use sql_migration_persistence::MIGRATION_TABLE_NAME;

use connection_wrapper::Connection;
use datamodel::Datamodel;
use flavour::SqlFlavour;
use migration_connector::*;
use quaint::prelude::ConnectionInfo;
use sql_database_migration_inferrer::*;
use sql_schema_describer::SqlSchema;
use tokio::sync::Mutex;

/// The top-level SQL migration connector.
pub struct SqlMigrationConnector {
    connection: Mutex<Option<Connection>>,
    connection_string: String,
}

impl SqlMigrationConnector {
    /// Construct the SQL migration connector and establish the default connection.
    pub async fn new(connection_string: &str) -> ConnectorResult<Self> {
        Ok(Self {
            connection_string: connection_string.to_owned(),
            connection: Default::default(),
        })
    }

    /// The SQL flavour without relation to an existing connection.
    fn abstract_flavour(&self) -> Box<dyn SqlFlavour + 'static> {
        todo!()
    }

    /// Create the database corresponding to the connection string, without initializing the connector.
    pub async fn create_database(database_str: &str) -> ConnectorResult<String> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let flavour = flavour::from_connection_info(&connection_info);
        flavour.create_database(database_str).await
    }

    /// Drop the database corresponding to the connection string, without initializing the connector.
    pub async fn drop_database(database_str: &str) -> ConnectorResult<()> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let flavour = flavour::from_connection_info(&connection_info);

        flavour.drop_database(database_str).await
    }

    /// Set up the database for connector-test-kit, without initializing the connector.
    pub async fn qe_setup(database_str: &str) -> ConnectorResult<()> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

        let flavour = flavour::from_connection_info(&connection_info);

        flavour.qe_setup(database_str).await
    }

    async fn conn(&self) -> ConnectorResult<Connection> {
        let connection = self.connection.lock().await;

        match connection.as_ref() {
            Some(connection) => Ok(connection.clone()),
            None => Ok(Connection::connect(&self.connection_string).await?),
        }
    }

    /// Describes the schema. Made public for tests.
    pub async fn describe_schema(&self) -> ConnectorResult<SqlSchema> {
        self.conn().await?.describe_schema().await
    }
}

#[async_trait::async_trait]
impl MigrationConnector for SqlMigrationConnector {
    type DatabaseMigration = SqlMigration;

    async fn acquire_advisory_lock(&self) -> ConnectorResult<()> {
        let connection = self.conn().await?;

        connection.acquire_advisory_lock().await?;

        Ok(())
    }

    fn connector_type(&self) -> &'static str {
        todo!()
        // self.default_connection.connection_info().sql_family().as_str()
    }

    async fn create_database(database_str: &str) -> ConnectorResult<String> {
        Self::create_database(database_str).await
    }

    async fn reset(&self) -> ConnectorResult<()> {
        let connection = self.conn().await?;

        Ok(connection.flavour().reset(&connection).await?)
    }

    /// Optionally check that the features implied by the provided datamodel are all compatible with
    /// the specific database version being used.
    fn check_database_version_compatibility(
        &self,
        datamodel: &Datamodel,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        self.flavour().check_database_version_compatibility(datamodel)
    }

    fn migration_persistence(&self) -> &dyn MigrationPersistence {
        self
    }

    fn database_migration_inferrer(&self) -> &dyn DatabaseMigrationInferrer<SqlMigration> {
        self
    }

    fn database_migration_step_applier(&self) -> &dyn DatabaseMigrationStepApplier<SqlMigration> {
        self
    }

    fn destructive_change_checker(&self) -> &dyn DestructiveChangeChecker<SqlMigration> {
        self
    }

    fn new_migration_persistence(&self) -> &dyn ImperativeMigrationsPersistence {
        self
    }

    async fn version(&self) -> ConnectorResult<String> {
        Ok(self
            .conn()
            .await?
            .version()
            .await?
            .unwrap_or_else(|| "Database version information not available.".into()))
    }
}
