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
use quaint::connector::{MysqlParams, PostgresParams};
use serde_json;
use sql_connection::{Mysql, Postgresql, Sqlite, SyncSqlConnection};
use sql_database_migration_inferrer::*;
use sql_database_step_applier::*;
use sql_destructive_changes_checker::*;
use sql_migration_persistence::*;
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::{convert::TryFrom, fs, path::PathBuf, sync::Arc};
use url::Url;

pub type Result<T> = std::result::Result<T, SqlError>;

#[allow(unused, dead_code)]
pub struct SqlMigrationConnector {
    pub url: String,
    pub file_path: Option<String>,
    pub sql_family: SqlFamily,
    pub schema_name: String,
    pub database: Arc<dyn SyncSqlConnection + Send + Sync + 'static>,
    pub migration_persistence: Arc<dyn MigrationPersistence>,
    pub database_migration_inferrer: Arc<dyn DatabaseMigrationInferrer<SqlMigration>>,
    pub database_migration_step_applier: Arc<dyn DatabaseMigrationStepApplier<SqlMigration>>,
    pub destructive_changes_checker: Arc<dyn DestructiveChangesChecker<SqlMigration>>,
    pub database_introspector: Arc<dyn SqlSchemaDescriberBackend + Send + Sync + 'static>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SqlFamily {
    Sqlite,
    Postgres,
    Mysql,
}

impl SqlFamily {
    fn connector_type_string(&self) -> &'static str {
        match self {
            SqlFamily::Postgres => "postgresql",
            SqlFamily::Mysql => "mysql",
            SqlFamily::Sqlite => "sqlite",
        }
    }
}

impl SqlMigrationConnector {
    pub fn postgres(url_str: &str, pooled: bool) -> crate::Result<Self> {
        let url = Url::parse(url_str)?;
        let params = PostgresParams::try_from(url.clone())?;

        let schema = params.schema.clone();

        let conn = if pooled {
            let pool = Postgresql::new_pooled(url.clone())?;

            // Postgres connection pools are lazy, we need to query to fail early when the database
            // is not reachable.
            pool.query_raw("SELECT 1", &[])?;

            pool
        } else {
            Postgresql::new_unpooled(url.clone())?
        };

        Ok(Self::create_connector(
            url_str,
            Arc::new(conn),
            SqlFamily::Postgres,
            schema,
            None,
        ))
    }

    pub fn mysql(url_str: &str, pooled: bool) -> crate::Result<Self> {
        let url = Url::parse(url_str)?;
        let params = MysqlParams::try_from(url.clone())?;

        let schema = params.dbname.clone();

        let conn = if pooled {
            Mysql::new_pooled(url)?
        } else {
            Mysql::new_unpooled(url)?
        };

        // Async MySQL connections are lazy - we have to run a query to confirm that the schema we
        // connected to exists.
        if !schema.is_empty() {
            conn.query_raw("SELECT 1 + 1", &[])?;
        }

        Ok(Self::create_connector(
            url_str,
            Arc::new(conn),
            SqlFamily::Mysql,
            schema,
            None,
        ))
    }

    pub fn sqlite(url: &str) -> crate::Result<Self> {
        let schema_name = "lift";
        let conn = Sqlite::new(url, schema_name)?;
        let file_path = conn.file_path().to_owned();

        Ok(Self::create_connector(
            url,
            Arc::new(conn),
            SqlFamily::Sqlite,
            schema_name.to_owned(),
            Some(file_path),
        ))
    }

    fn create_connector(
        url: &str,
        conn: Arc<dyn SyncSqlConnection + Send + Sync + 'static>,
        sql_family: SqlFamily,
        schema_name: String,
        file_path: Option<String>,
    ) -> Self {
        let inspector: Arc<dyn SqlSchemaDescriberBackend + Send + Sync + 'static> = match sql_family {
            SqlFamily::Mysql => Arc::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(Arc::clone(&conn))),
            SqlFamily::Postgres => Arc::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(Arc::clone(
                &conn,
            ))),
            SqlFamily::Sqlite => Arc::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(Arc::clone(&conn))),
        };

        let migration_persistence = Arc::new(SqlMigrationPersistence {
            sql_family,
            connection: Arc::clone(&conn),
            schema_name: schema_name.clone(),
            file_path: file_path.clone(),
        });

        let database_migration_inferrer = Arc::new(SqlDatabaseMigrationInferrer {
            sql_family,
            introspector: Arc::clone(&inspector),
            schema_name: schema_name.to_string(),
        });

        let database_migration_step_applier = Arc::new(SqlDatabaseStepApplier {
            sql_family,
            schema_name: schema_name.clone(),
            conn: Arc::clone(&conn),
        });

        let destructive_changes_checker = Arc::new(SqlDestructiveChangesChecker {
            schema_name: schema_name.clone(),
            database: Arc::clone(&conn),
        });

        Self {
            url: url.to_string(),
            file_path,
            sql_family,
            schema_name,
            database: Arc::clone(&conn),
            migration_persistence,
            database_migration_inferrer,
            database_migration_step_applier,
            destructive_changes_checker,
            database_introspector: Arc::clone(&inspector),
        }
    }
}

impl MigrationConnector for SqlMigrationConnector {
    type DatabaseMigration = SqlMigration;

    fn connector_type(&self) -> &'static str {
        self.sql_family.connector_type_string()
    }

    fn create_database(&self, db_name: &str) -> ConnectorResult<()> {
        match self.sql_family {
            SqlFamily::Postgres => {
                self.database
                    .query_raw(&format!("CREATE DATABASE \"{}\"", db_name), &[])?;

                Ok(())
            }
            SqlFamily::Sqlite => Ok(()),
            SqlFamily::Mysql => {
                self.database
                    .query_raw(&format!("CREATE DATABASE `{}`", db_name), &[])?;

                Ok(())
            }
        }
    }

    fn initialize(&self) -> ConnectorResult<()> {
        // TODO: this code probably does not ever do anything. The schema/db creation happens already in the helper functions above.
        match self.sql_family {
            SqlFamily::Sqlite => {
                if let Some(file_path) = &self.file_path {
                    let path_buf = PathBuf::from(&file_path);
                    match path_buf.parent() {
                        Some(parent_directory) => {
                            fs::create_dir_all(parent_directory).expect("creating the database folders failed")
                        }
                        None => {}
                    }
                }
            }
            SqlFamily::Postgres => {
                let schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", &self.schema_name);

                debug!("{}", schema_sql);

                self.database.query_raw(&schema_sql, &[])?;
            }
            SqlFamily::Mysql => {
                let schema_sql = format!(
                    "CREATE SCHEMA IF NOT EXISTS `{}` DEFAULT CHARACTER SET latin1;",
                    &self.schema_name
                );

                debug!("{}", schema_sql);

                self.database.query_raw(&schema_sql, &[])?;
            }
        }

        self.migration_persistence.init();

        Ok(())
    }

    fn reset(&self) -> ConnectorResult<()> {
        self.migration_persistence.reset();
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
