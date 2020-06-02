mod component;
mod database_info;
mod datamodel_helpers;
mod error;
mod sql_database_migration_inferrer;
mod sql_database_step_applier;
mod sql_destructive_changes_checker;
mod sql_migration;
mod sql_migration_persistence;
mod sql_renderer;
mod sql_schema_calculator;
mod sql_schema_differ;
mod sql_schema_helpers;

pub use error::*;
pub use sql_migration::*;
pub use sql_migration_persistence::MIGRATION_TABLE_NAME;

use component::Component;
use database_info::DatabaseInfo;
use migration_connector::*;
use quaint::{
    error::ErrorKind,
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};
use sql_database_migration_inferrer::*;
use sql_database_step_applier::*;
use sql_destructive_changes_checker::*;
use sql_migration_persistence::*;
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::{
    collections::HashMap,
    fs,
    path::{self, PathBuf},
    sync::Arc,
    time::Duration,
};
use tracing::debug;
use url::Url;
use user_facing_errors::migration_engine::DatabaseMigrationFormatChanged;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub struct SqlMigrationConnector {
    pub schema_name: String,
    pub database: Arc<dyn Queryable + Send + Sync + 'static>,
    pub database_info: DatabaseInfo,
    pub database_describer: Box<dyn SqlSchemaDescriberBackend + Send + Sync + 'static>,
}

impl SqlMigrationConnector {
    pub async fn new(database_str: &str) -> ConnectorResult<Self> {
        let (connection, database_info) = connect(database_str).await?;
        let schema_name = connection.connection_info().schema_name().to_owned();
        let conn = Arc::new(connection) as Arc<dyn Queryable + Send + Sync>;

        let describer: Box<dyn SqlSchemaDescriberBackend + Send + Sync + 'static> = match database_info.sql_family() {
            SqlFamily::Mysql => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(Arc::clone(&conn))),
            SqlFamily::Postgres => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(Arc::clone(
                &conn,
            ))),
            SqlFamily::Sqlite => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(Arc::clone(&conn))),
        };

        Ok(Self {
            database_info,
            schema_name,
            database: conn,
            database_describer: describer,
        })
    }

    pub async fn create_database(database_str: &str) -> ConnectorResult<String> {
        use anyhow::Context;
        use futures::future::TryFutureExt;

        match ConnectionInfo::from_url(database_str).ok() {
            Some(ConnectionInfo::Postgres(postgres_url)) => {
                let url = Url::parse(database_str).unwrap();
                let db_name = postgres_url.dbname();

                let (conn, _) = create_postgres_admin_conn(url).await?;

                let query = format!("CREATE DATABASE \"{}\"", db_name);
                catch(
                    conn.connection_info(),
                    conn.query_raw(&query, &[]).map_err(SqlError::from),
                )
                .await?;

                Ok(db_name.to_owned())
            }
            Some(ConnectionInfo::Sqlite { file_path, .. }) => {
                let path = path::Path::new(&file_path);
                if path.exists() {
                    return Ok(file_path);
                }

                let dir = path.parent();

                if let Some((dir, false)) = dir.map(|dir| (dir, dir.exists())) {
                    std::fs::create_dir_all(dir)
                        .context("Creating SQLite database parent directory.")
                        .map_err(|io_err| {
                            ConnectorError::from_kind(migration_connector::ErrorKind::Generic(io_err.into()))
                        })?;
                }

                connect(database_str).await?;

                Ok(file_path)
            }
            Some(ConnectionInfo::Mysql(mysql_url)) => {
                let mut url = Url::parse(database_str).unwrap();
                url.set_path("/mysql");
                let (conn, _) = connect(&url.to_string()).await?;

                let db_name = mysql_url.dbname();

                let query = format!("CREATE DATABASE `{}`", db_name);
                catch(
                    conn.connection_info(),
                    conn.query_raw(&query, &[]).map_err(SqlError::from),
                )
                .await?;

                Ok(db_name.to_owned())
            }
            None => unreachable!(
                "Invalid URL or unsupported connector in the datasource ({:?})",
                database_str
            ),
        }
    }

    async fn initialize_impl(&self) -> SqlResult<()> {
        match self.database_info.connection_info() {
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

    fn connection_info(&self) -> &ConnectionInfo {
        self.database_info.connection_info()
    }

    async fn drop_database(&self) -> ConnectorResult<()> {
        use quaint::ast::Value;

        catch(self.connection_info(), async {
            match &self.connection_info() {
                ConnectionInfo::Postgres(_) => {
                    let sql_str = format!(r#"DROP SCHEMA "{}" CASCADE;"#, self.schema_name());
                    debug!("{}", sql_str);

                    self.conn().query_raw(&sql_str, &[]).await.ok();
                }
                ConnectionInfo::Sqlite { file_path, .. } => {
                    self.conn()
                        .query_raw("DETACH DATABASE ?", &[Value::from(self.schema_name())])
                        .await
                        .ok();
                    std::fs::remove_file(file_path).ok(); // ignore potential errors
                    self.conn()
                        .query_raw(
                            "ATTACH DATABASE ? AS ?",
                            &[Value::from(file_path.as_str()), Value::from(self.schema_name())],
                        )
                        .await?;
                }
                ConnectionInfo::Mysql(_) => {
                    let sql_str = format!(r#"DROP SCHEMA `{}`;"#, self.schema_name());
                    debug!("{}", sql_str);
                    self.conn().query_raw(&sql_str, &[]).await?;
                }
            };

            Ok(())
        })
        .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl MigrationConnector for SqlMigrationConnector {
    type DatabaseMigration = SqlMigration;

    fn connector_type(&self) -> &'static str {
        self.connection_info().sql_family().as_str()
    }

    async fn create_database(database_str: &str) -> ConnectorResult<String> {
        Self::create_database(database_str).await
    }

    async fn initialize(&self) -> ConnectorResult<()> {
        catch(self.connection_info(), self.initialize_impl()).await?;

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

    fn destructive_changes_checker<'a>(&'a self) -> Box<dyn DestructiveChangesChecker<SqlMigration> + 'a> {
        Box::new(SqlDestructiveChangesChecker { connector: self })
    }

    fn deserialize_database_migration(
        &self,
        json: serde_json::Value,
    ) -> Result<SqlMigration, DatabaseMigrationFormatChanged> {
        serde_json::from_value(json).map_err(|_| DatabaseMigrationFormatChanged)
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
            .query_raw("SELECT 1", &[])
            .await
            .map_err(SqlError::from)
            .map_err(|err| err.into_connector_error(&connection.connection_info()))?;

        Ok(connection)
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

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
async fn create_postgres_admin_conn(mut url: Url) -> ConnectorResult<(Quaint, DatabaseInfo)> {
    use migration_connector::ErrorKind;

    let candidate_default_databases = &["postgres", "template1"];

    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params: Vec<String> = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let params: String = params.join("&");
    url.set_query(Some(&params));

    let mut conn = None;

    for database_name in candidate_default_databases {
        url.set_path(&format!("/{}", database_name));
        match connect(url.as_str()).await {
            // If the database does not exist, try the next one.
            Err(err) => match &err.kind {
                migration_connector::ErrorKind::DatabaseDoesNotExist { .. } => (),
                _other_outcome => {
                    conn = Some(Err(err));
                    break;
                }
            },
            // If the outcome is anything else, use this.
            other_outcome => {
                conn = Some(other_outcome);
                break;
            }
        }
    }

    let conn = conn
        .ok_or_else(|| {
            ConnectorError::from_kind(ErrorKind::DatabaseCreationFailed {
                explanation: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned()
            })
        })??;

    Ok(conn)
}
