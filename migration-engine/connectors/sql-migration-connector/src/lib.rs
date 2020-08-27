#![deny(rust_2018_idioms, unsafe_code)]
#![allow(clippy::trivial_regex)] // these will grow

// This is public for test purposes.
pub mod sql_migration;

mod component;
mod database_info;
mod error;
mod flavour;
mod sql_database_migration_inferrer;
mod sql_database_step_applier;
mod sql_destructive_change_checker;
mod sql_migration_persistence;
mod sql_renderer;
mod sql_schema_calculator;
mod sql_schema_differ;

pub use error::{SqlError, SqlResult};
pub use sql_migration_persistence::MIGRATION_TABLE_NAME;

use component::Component;
use database_info::DatabaseInfo;
use flavour::SqlFlavour;
use migration_connector::*;
use quaint::{
    error::ErrorKind,
    prelude::{ConnectionInfo, Queryable},
    single::Quaint,
};
use sql_database_migration_inferrer::*;
use sql_database_step_applier::*;
use sql_destructive_change_checker::*;
use sql_migration::SqlMigration;
use sql_migration_persistence::*;
use sql_schema_describer::SqlSchema;
use std::{sync::Arc, time::Duration};

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub struct SqlMigrationConnector {
    pub database: Arc<dyn Queryable + Send + Sync + 'static>,
    pub database_info: DatabaseInfo,
    flavour: Box<dyn SqlFlavour + Send + Sync + 'static>,
}

impl SqlMigrationConnector {
    pub async fn new(database_str: &str) -> ConnectorResult<Self> {
        let (connection, database_info) = connect(database_str).await?;
        let flavour = flavour::from_connection_info(database_info.connection_info());
        flavour.check_database_info(&database_info)?;

        Ok(Self {
            flavour,
            database_info,
            database: Arc::new(connection),
        })
    }

    pub async fn create_database(database_str: &str) -> ConnectorResult<String> {
        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;
        let flavour = flavour::from_connection_info(&connection_info);
        flavour.create_database(database_str).await
    }

    async fn drop_database(&self) -> ConnectorResult<()> {
        catch(
            self.database_info().connection_info(),
            self.flavour().drop_database(self.conn(), self.schema_name()),
        )
        .await
    }

    async fn describe_schema(&self) -> SqlResult<SqlSchema> {
        let conn = self.connector().database.clone();
        let schema_name = self.schema_name();

        self.flavour.describe_schema(schema_name, conn).await
    }
}

#[async_trait::async_trait]
impl MigrationConnector for SqlMigrationConnector {
    type DatabaseMigration = SqlMigration;

    fn connector_type(&self) -> &'static str {
        self.database_info.connection_info().sql_family().as_str()
    }

    async fn create_database(database_str: &str) -> ConnectorResult<String> {
        Self::create_database(database_str).await
    }

    async fn initialize(&self) -> ConnectorResult<()> {
        catch(
            self.database_info.connection_info(),
            self.flavour.initialize(self.database.as_ref(), &self.database_info),
        )
        .await?;

        self.migration_persistence().init().await?;

        Ok(())
    }

    async fn reset(&self) -> ConnectorResult<()> {
        self.migration_persistence().reset().await?;
        self.drop_database().await?;

        Ok(())
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
}

pub(crate) async fn catch<O>(
    connection_info: &ConnectionInfo,
    fut: impl std::future::Future<Output = Result<O, SqlError>>,
) -> Result<O, ConnectorError> {
    match fut.await {
        Ok(o) => Ok(o),
        Err(sql_error) => Err(sql_error.into_connector_error(connection_info)),
    }
}

async fn connect(database_str: &str) -> ConnectorResult<(Quaint, DatabaseInfo)> {
    let connection_info =
        ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

    let connection_fut = async {
        let connection = Quaint::new(database_str)
            .await
            .map_err(SqlError::from)
            .map_err(|err: SqlError| err.into_connector_error(&connection_info))?;

        // async connections can be lazy, so we issue a simple query to fail early if the database
        // is not reachable.
        connection
            .raw_cmd("SELECT 1")
            .await
            .map_err(SqlError::from)
            .map_err(|err| err.into_connector_error(&connection.connection_info()))?;

        Ok::<_, ConnectorError>(connection)
    };

    let connection = tokio::time::timeout(CONNECTION_TIMEOUT, connection_fut)
        .await
        .map_err(|_elapsed| {
            // TODO: why...
            SqlError::from(ErrorKind::ConnectTimeout("Tokio timer".into())).into_connector_error(&connection_info)
        })??;

    let database_info = DatabaseInfo::new(&connection, connection.connection_info().clone())
        .await
        .map_err(|sql_error| sql_error.into_connector_error(&connection_info))?;

    Ok((connection, database_info))
}
