#![deny(rust_2018_idioms, unsafe_code)]
#![allow(clippy::trivial_regex)] // these will grow

// This is public for test purposes.
pub mod sql_migration;

mod component;
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
use error::{SqlError, SqlResult};
pub use sql_migration_persistence::MIGRATION_TABLE_NAME;

use component::Component;
use database_info::DatabaseInfo;
use flavour::SqlFlavour;
use migration_connector::*;
use quaint::{prelude::ConnectionInfo, single::Quaint};
use sql_database_migration_inferrer::*;
use sql_database_step_applier::*;
use sql_destructive_change_checker::*;
use sql_migration::SqlMigration;
use sql_migration_persistence::*;
use sql_schema_describer::SqlSchema;

pub struct SqlMigrationConnector {
    connection: Connection,
    database_info: DatabaseInfo,
    flavour: Box<dyn SqlFlavour + Send + Sync + 'static>,
}

impl SqlMigrationConnector {
    pub async fn new(database_str: &str) -> ConnectorResult<Self> {
        let (connection, database_info) = connect(database_str).await?;
        let flavour = flavour::from_connection_info(database_info.connection_info());

        flavour.check_database_info(&database_info)?;
        flavour.ensure_connection_validity(&connection).await?;

        Ok(Self {
            flavour,
            database_info,
            connection,
        })
    }

    pub async fn create_database(database_str: &str) -> ConnectorResult<String> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let flavour = flavour::from_connection_info(&connection_info);
        flavour.create_database(database_str).await
    }

    pub async fn qe_setup(database_str: &str) -> ConnectorResult<()> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

        let flavour = flavour::from_connection_info(&connection_info);

        flavour.qe_setup(database_str).await
    }

    /// For tests.
    pub fn quaint(&self) -> &Quaint {
        self.connection.quaint()
    }

    async fn describe_schema(&self) -> ConnectorResult<SqlSchema> {
        self.flavour.describe_schema(&self.connection).await
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
            .unwrap_or("Database version information not available.".into())
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

    fn migration_persistence<'a>(&'a self) -> Box<dyn MigrationPersistence + 'a> {
        Box::new(SqlMigrationPersistence { connector: self })
    }

    fn database_migration_inferrer<'a>(&'a self) -> Box<dyn DatabaseMigrationInferrer<SqlMigration> + 'a> {
        Box::new(SqlDatabaseMigrationInferrer { connector: self })
    }

    fn database_migration_step_applier<'a>(&'a self) -> Box<dyn DatabaseMigrationStepApplier<SqlMigration> + 'a> {
        Box::new(SqlDatabaseStepApplier { connector: self })
    }

    fn destructive_change_checker<'a>(&'a self) -> Box<dyn DestructiveChangeChecker<SqlMigration> + 'a> {
        Box::new(SqlDestructiveChangeChecker { connector: self })
    }

    fn deserialize_database_migration(&self, json: serde_json::Value) -> Option<SqlMigration> {
        serde_json::from_value(json).ok()
    }

    fn new_migration_persistence(&self) -> &dyn ImperativeMigrationsPersistence {
        self
    }
}

async fn connect(database_str: &str) -> ConnectorResult<(Connection, DatabaseInfo)> {
    let connection_info =
        ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

    let connection = Quaint::new(database_str)
        .await
        .map_err(SqlError::from)
        .map_err(|err: SqlError| err.into_connector_error(&connection_info))?;

    let database_info = DatabaseInfo::new(&connection, connection.connection_info().clone())
        .await
        .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;

    Ok((Connection::new(connection), database_info))
}
