use super::misc_helpers::*;
use introspection_connector::{DatabaseMetadata, IntrospectionConnector};
use quaint::{single::Quaint, prelude::{Queryable, SqlFamily}};
use sql_introspection_connector::SqlIntrospectionConnector;
use std::sync::Arc;
use test_setup::*;

pub struct TestApi {
    db_name: &'static str,
    sql_family: SqlFamily,
    database: Arc<dyn Queryable + Send + Sync + 'static>,
    introspection_connector: SqlIntrospectionConnector,
}

impl TestApi {
    pub async fn list_databases(&self) -> Vec<String> {
        self.introspection_connector.list_databases().await.unwrap()
    }

    pub async fn introspect(&self) -> String {
        let datamodel = self.introspection_connector.introspect().await.unwrap();
        datamodel::render_datamodel_to_string(&datamodel).expect("Datamodel rendering failed")
    }

    pub async fn get_metadata(&self) -> DatabaseMetadata {
        let metadata = self.introspection_connector.get_metadata().await.unwrap();
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

    pub fn db_name(&self) -> &str {
        self.db_name.as_ref()
    }
}

pub async fn mysql_test_api(db_name: &'static str) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(db_name);
    let url = mysql_url(db_name.as_ref());
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();

    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        db_name,
        database: Arc::new(conn),
        sql_family: SqlFamily::Mysql,
        introspection_connector,
    }
}

pub async fn postgres_test_api(db_name: &'static str) -> TestApi {
    let url = postgres_10_url(db_name);
    let database = test_setup::create_postgres_database(&url.parse().unwrap()).await.unwrap();

    let drop_schema = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", SCHEMA_NAME);
    database.query_raw(&drop_schema, &[]).await.ok();

    let create_schema = format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", SCHEMA_NAME);
    database.query_raw(&create_schema, &[]).await.ok();

    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        db_name,
        database: Arc::new(database),
        sql_family: SqlFamily::Postgres,
        introspection_connector: introspection_connector,
    }
}

pub async fn sqlite_test_api(db_name: &'static str) -> TestApi {
    let database_file_path = sqlite_test_file(db_name);
    std::fs::remove_file(database_file_path.clone()).ok(); // ignore potential errors

    let connection_string = sqlite_test_url(db_name);

    let database = Quaint::new(&connection_string).await.unwrap();

    let introspection_connector = SqlIntrospectionConnector::new(&connection_string).await.unwrap();

    TestApi {
        db_name,
        database: Arc::new(database),
        sql_family: SqlFamily::Sqlite,
        introspection_connector,
    }
}
