//! The SQL migration connector.

#![deny(rust_2018_idioms, unsafe_code, missing_docs)]
#![allow(clippy::trivial_regex)] // these will grow

// This is public for test purposes.
#[allow(missing_docs)]
pub mod sql_migration;

mod connection_wrapper;
mod database_info;
mod error;
mod flavour;
mod sql_database_migration_inferrer;
mod sql_database_step_applier;
mod sql_destructive_change_checker;
mod sql_imperative_migration_persistence;
mod sql_migration_persistence;
mod sql_renderer;
mod sql_schema_calculator;
mod sql_schema_differ;

use connection_wrapper::Connection;
use error::quaint_error_to_connector_error;
pub use sql_migration_persistence::MIGRATION_TABLE_NAME;

use database_info::DatabaseInfo;
use flavour::SqlFlavour;
use migration_connector::*;
use quaint::{prelude::ConnectionInfo, prelude::SqlFamily, single::Quaint};
use sql_database_migration_inferrer::*;
use sql_migration::SqlMigration;
use sql_schema_describer::SqlSchema;

/// The top-level SQL migration connector.
pub struct SqlMigrationConnector {
    connection: Connection,
    database_info: DatabaseInfo,
    flavour: Box<dyn SqlFlavour + Send + Sync + 'static>,
}

impl SqlMigrationConnector {
    /// Construct and initialize the SQL migration connector.
    pub async fn new(database_str: &str) -> ConnectorResult<Self> {
        let connection = connect(database_str).await?;
        let database_info = DatabaseInfo::new(connection.quaint(), connection.connection_info().clone()).await?;
        let flavour = flavour::from_connection_info(database_info.connection_info());

        flavour.check_database_info(&database_info)?;
        flavour.ensure_connection_validity(&connection).await?;

        Ok(Self {
            flavour,
            database_info,
            connection,
        })
    }

    /// Create the database corresponding to the connection string, without initializing the connector.
    pub async fn create_database(database_str: &str) -> ConnectorResult<String> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let flavour = flavour::from_connection_info(&connection_info);
        flavour.create_database(database_str).await
    }

    /// Set up the database for connector-test-kit, without initializing the connector.
    pub async fn qe_setup(database_str: &str) -> ConnectorResult<()> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

        let flavour = flavour::from_connection_info(&connection_info);

        flavour.qe_setup(database_str).await
    }

    fn conn(&self) -> &Connection {
        &self.connection
    }

    fn database_info(&self) -> &DatabaseInfo {
        &self.database_info
    }

    fn flavour(&self) -> &(dyn SqlFlavour + Send + Sync) {
        self.flavour.as_ref()
    }

    /// For tests.
    pub fn quaint(&self) -> &Quaint {
        self.connection.quaint()
    }

    /// Made public for tests.
    pub async fn describe_schema(&self) -> ConnectorResult<SqlSchema> {
        self.flavour.describe_schema(&self.connection).await
    }

    fn schema_name(&self) -> &str {
        self.database_info.connection_info().schema_name()
    }

    fn sql_family(&self) -> SqlFamily {
        self.database_info.sql_family()
    }
}

#[async_trait::async_trait]
impl MigrationConnector for SqlMigrationConnector {
    type DatabaseMigration = SqlMigration;

    fn connector_type(&self) -> &'static str {
        self.database_info.connection_info().sql_family().as_str()
    }

    fn version(&self) -> String {
        self.database_info
            .database_version
            .clone()
            .unwrap_or_else(|| "Database version information not available.".into())
    }

    async fn create_database(database_str: &str) -> ConnectorResult<String> {
        Self::create_database(database_str).await
    }

    async fn initialize(&self) -> ConnectorResult<()> {
        self.migration_persistence().init().await?;

        Ok(())
    }

    async fn reset(&self) -> ConnectorResult<()> {
        self.flavour.reset(self.conn()).await
    }

    /// Optionally check that the features implied by the provided datamodel are all compatible with
    /// the specific database version being used.
    fn check_database_version_compatibility(&self, datamodel: &datamodel::dml::Datamodel) -> Vec<MigrationError> {
        self.database_info.check_database_version_compatibility(datamodel)
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

    fn deserialize_database_migration(&self, json: serde_json::Value) -> Option<SqlMigration> {
        serde_json::from_value(json).ok()
    }

    fn new_migration_persistence(&self) -> &dyn ImperativeMigrationsPersistence {
        self
    }
}

async fn connect(database_str: &str) -> ConnectorResult<Connection> {
    let connection_info =
        ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

    let connection = Quaint::new(database_str)
        .await
        .map_err(|err| quaint_error_to_connector_error(err, &connection_info))?;

    Ok(Connection::new(connection))
}
