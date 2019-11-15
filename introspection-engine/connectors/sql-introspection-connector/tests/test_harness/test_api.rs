use super::misc_helpers::*;
use introspection_connector::{DatabaseMetadata, IntrospectionConnector};
use quaint::prelude::SqlFamily;
use sql_connection::SyncSqlConnection;
use sql_introspection_connector::SqlIntrospectionConnector;
use std::sync::Arc;

pub struct TestApi {
    sql_family: SqlFamily,
    database: Arc<dyn SyncSqlConnection + Send + Sync + 'static>,
    introspection_connector: SqlIntrospectionConnector,
}

impl TestApi {
    pub fn list_databases(&self) -> Vec<String> {
        self.introspection_connector.list_databases().unwrap()
    }

    pub fn introspect(&self) -> String {
        let datamodel = self.introspection_connector.introspect(SCHEMA_NAME).unwrap();
        datamodel::render_datamodel_to_string(&datamodel).expect("Datamodel rendering failed")
    }

    pub fn get_metadata(&self) -> DatabaseMetadata {
        let metadata = self.introspection_connector.get_metadata(SCHEMA_NAME).unwrap();
        metadata
    }

    pub fn sql_family(&self) -> SqlFamily {
        self.sql_family
    }

    pub fn barrel(&self) -> BarrelMigrationExecutor {
        BarrelMigrationExecutor {
            database: Arc::clone(&self.database),
            sql_variant: match self.sql_family {
                SqlFamily::Mysql => barrel::SqlVariant::Mysql,
                SqlFamily::Postgres => barrel::SqlVariant::Pg,
                SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
            },
        }
    }
}

pub fn mysql_test_api() -> TestApi {
    let database = database(&mysql_url());

    let drop_database = dbg!(format!("DROP DATABASE IF EXISTS `{}`;", SCHEMA_NAME));
    database.query_raw(&drop_database, &[]).ok();
    let create_database = dbg!(format!("CREATE DATABASE `{}`;", SCHEMA_NAME));
    database.query_raw(&create_database, &[]).ok();

    let introspection_connector = SqlIntrospectionConnector::new(&mysql_url()).unwrap();

    TestApi {
        database: database.into(),
        sql_family: SqlFamily::Mysql,
        introspection_connector,
    }
}

pub fn postgres_test_api() -> TestApi {
    let database = database(&postgres_url());

    let drop_schema = dbg!(format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", SCHEMA_NAME));
    database.query_raw(&drop_schema, &[]).ok();

    let create_schema = dbg!(format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", SCHEMA_NAME));
    database.query_raw(&create_schema, &[]).ok();

    let introspection_connector = SqlIntrospectionConnector::new(&postgres_url()).unwrap();

    TestApi {
        database: database.into(),
        sql_family: SqlFamily::Postgres,
        introspection_connector: introspection_connector,
    }
}

pub fn sqlite_test_api() -> TestApi {
    let database = database(&sqlite_test_url());

    let database_file_path = sqlite_test_file();
    std::fs::remove_file(database_file_path.clone()).ok(); // ignore potential errors
    let introspection_connector = SqlIntrospectionConnector::new(&sqlite_test_url()).unwrap();

    TestApi {
        database: database.into(),
        sql_family: SqlFamily::Sqlite,
        introspection_connector,
    }
}
