#[macro_use]
extern crate log;

mod error;
mod sql_database_migration_inferrer;
mod sql_database_step_applier;
mod sql_destructive_changes_checker;
mod sql_migration;
mod sql_migration_persistence;
mod sql_renderer;
mod sql_schema_calculator;
mod sql_schema_differ;

pub use error::*;
pub use sql_migration::*;

use migration_connector::*;
use quaint::{
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};
use sql_database_migration_inferrer::*;
use sql_database_step_applier::*;
use sql_destructive_changes_checker::*;
use sql_migration_persistence::*;
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::{fs, path::PathBuf, sync::Arc, time::Duration};

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub struct SqlMigrationConnector {
    pub connection_info: ConnectionInfo,
    pub schema_name: String,
    pub database: Arc<dyn Queryable + Send + Sync + 'static>,
    pub migration_persistence: Arc<dyn MigrationPersistence>,
    pub database_migration_inferrer: Arc<dyn DatabaseMigrationInferrer<SqlMigration>>,
    pub database_migration_step_applier: Arc<dyn DatabaseMigrationStepApplier<SqlMigration>>,
    pub destructive_changes_checker: Arc<dyn DestructiveChangesChecker<SqlMigration>>,
    pub database_describer: Arc<dyn SqlSchemaDescriberBackend + Send + Sync + 'static>,
}

impl SqlMigrationConnector {
    pub async fn new(database_str: &str, provider: &str) -> ConnectorResult<Self> {
        validate_database_str(database_str, provider)?;

        let connection_info =
            ConnectionInfo::from_url(database_str).map_err(|err| ConnectorError::url_parse_error(err, database_str))?;

        let connection_fut = async {
            let connection = Quaint::new(database_str)
                .await
                .map_err(SqlError::from)
                .map_err(|err| err.into_connector_error(&connection_info))?;

            // async connections can be lazy, so we issue a simple query to fail early if the database
            // is not reachable.
            connection
                .query_raw("SELECT 1", &[])
                .await
                .map_err(SqlError::from)
                .map_err(|err| err.into_connector_error(&connection.connection_info()))?;

            Ok(connection)
        };

        let connection = tokio::time::timeout(CONNECTION_TIMEOUT, connection_fut)
            .await
            .map_err(|_elapsed| {
                SqlError::from(quaint::error::Error::ConnectTimeout).into_connector_error(&connection_info)
            })??;

        let schema_name = connection.connection_info().schema_name().to_owned();

        let sql_family = connection.connection_info().sql_family();
        let connection_info = connection.connection_info().clone();

        let conn = Arc::new(connection) as Arc<dyn Queryable + Send + Sync>;

        let describer: Arc<dyn SqlSchemaDescriberBackend + Send + Sync + 'static> = match sql_family {
            SqlFamily::Mysql => Arc::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(Arc::clone(&conn))),
            SqlFamily::Postgres => Arc::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(Arc::clone(
                &conn,
            ))),
            SqlFamily::Sqlite => Arc::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(Arc::clone(&conn))),
        };

        let migration_persistence = Arc::new(SqlMigrationPersistence {
            connection_info: connection_info.clone(),
            connection: Arc::clone(&conn),
            schema_name: schema_name.clone(),
        });

        let database_migration_inferrer = Arc::new(SqlDatabaseMigrationInferrer {
            connection_info: connection_info.clone(),
            describer: Arc::clone(&describer),
            schema_name: schema_name.to_string(),
        });

        let database_migration_step_applier = Arc::new(SqlDatabaseStepApplier {
            connection_info: connection_info.clone(),
            schema_name: schema_name.clone(),
            conn: Arc::clone(&conn),
        });

        let destructive_changes_checker = Arc::new(SqlDestructiveChangesChecker {
            connection_info: connection_info.clone(),
            schema_name: schema_name.clone(),
            database: Arc::clone(&conn),
            database_describer: describer.clone(),
        });

        Ok(Self {
            connection_info,
            schema_name,
            database: Arc::clone(&conn),
            migration_persistence,
            database_migration_inferrer,
            database_migration_step_applier,
            destructive_changes_checker,
            database_describer: Arc::clone(&describer),
        })
    }

    async fn create_database_impl(&self, db_name: &str) -> SqlResult<()> {
        match self.connection_info.sql_family() {
            SqlFamily::Postgres => {
                let query = format!("CREATE DATABASE \"{}\"", db_name);
                self.database.query_raw(&query, &[]).await?;

                Ok(())
            }
            SqlFamily::Sqlite => Ok(()),
            SqlFamily::Mysql => {
                let query = format!("CREATE DATABASE `{}`", db_name);
                self.database.query_raw(&query, &[]).await?;

                Ok(())
            }
        }
    }

    async fn initialize_impl(&self) -> SqlResult<()> {
        // TODO: this code probably does not ever do anything. The schema/db creation happens already in the helper functions above.
        match &self.connection_info {
            ConnectionInfo::Sqlite { file_path, .. } => {
                let path_buf = PathBuf::from(&file_path);
                match path_buf.parent() {
                    Some(parent_directory) => {
                        fs::create_dir_all(parent_directory).expect("creating the database folders failed")
                    }
                    None => {}
                }
            }
            ConnectionInfo::Postgres(_) => {
                let schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", &self.schema_name);

                debug!("{}", schema_sql);

                self.database.query_raw(&schema_sql, &[]).await?;
            }
            ConnectionInfo::Mysql(_) => {
                let schema_sql = format!(
                    "CREATE SCHEMA IF NOT EXISTS `{}` DEFAULT CHARACTER SET latin1;",
                    &self.schema_name
                );

                debug!("{}", schema_sql);

                self.database.query_raw(&schema_sql, &[]).await?;
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl MigrationConnector for SqlMigrationConnector {
    type DatabaseMigration = SqlMigration;

    fn connector_type(&self) -> &'static str {
        self.connection_info.sql_family().as_str()
    }

    async fn create_database(&self, db_name: &str) -> ConnectorResult<()> {
        catch(&self.connection_info, self.create_database_impl(db_name)).await
    }

    async fn initialize(&self) -> ConnectorResult<()> {
        catch(&self.connection_info, self.initialize_impl()).await?;

        self.migration_persistence().init().await?;

        Ok(())
    }

    async fn reset(&self) -> ConnectorResult<()> {
        self.migration_persistence().reset().await?;
        Ok(())
    }

    fn migration_persistence(&self) -> Arc<dyn MigrationPersistence> {
        Arc::clone(&self.migration_persistence)
    }

    fn database_migration_inferrer(&self) -> Arc<dyn DatabaseMigrationInferrer<SqlMigration>> {
        Arc::clone(&self.database_migration_inferrer)
    }

    fn database_migration_step_applier(&self) -> Arc<dyn DatabaseMigrationStepApplier<SqlMigration>> {
        Arc::clone(&self.database_migration_step_applier)
    }

    fn destructive_changes_checker(&self) -> Arc<dyn DestructiveChangesChecker<SqlMigration>> {
        Arc::clone(&self.destructive_changes_checker)
    }

    fn deserialize_database_migration(&self, json: serde_json::Value) -> SqlMigration {
        serde_json::from_value(json).expect("Deserializing the database migration failed.")
    }
}

pub(crate) async fn catch<O>(
    connection_info: &ConnectionInfo,
    fut: impl std::future::Future<Output = std::result::Result<O, SqlError>>,
) -> std::result::Result<O, migration_connector::ConnectorError> {
    match fut.await {
        Ok(o) => Ok(o),
        Err(sql_error) => Err(sql_error.into_connector_error(connection_info)),
    }
}

fn validate_database_str(database_str: &str, provider: &str) -> ConnectorResult<()> {
    let scheme = database_str.split(":").next();

    match (provider, scheme) {
        ("mysql", Some("mysql")) => Ok(()),
        ("postgresql", Some(scheme)) if scheme.starts_with("postgres") => Ok(()),
        ("postgres", Some(scheme)) if scheme.starts_with("postgres") => Ok(()),
        ("sqlite", Some("file")) | ("sqlite", Some("sqlite")) => Ok(()),
        _ => {
            let error = ConnectorError {
                kind: migration_connector::ErrorKind::InvalidDatabaseUrl,
                user_facing_error: None,
            };

            Err(error)
        }
    }
}
