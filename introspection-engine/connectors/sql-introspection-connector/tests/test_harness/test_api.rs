use super::misc_helpers::*;
use introspection_connector::{DatabaseMetadata, IntrospectionConnector};
use quaint::{single::Quaint, prelude::{Queryable, SqlFamily}};
use sql_introspection_connector::SqlIntrospectionConnector;
use std::sync::Arc;
use test_setup::*;

pub struct TestApi {
    db_name: &'static str,
    connection_info: quaint::prelude::ConnectionInfo,
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

    pub fn schema_name(&self) -> &str {
        self.connection_info.schema_name()
    }

    pub fn barrel(&self) -> BarrelMigrationExecutor {
        BarrelMigrationExecutor {
            schema_name: self.schema_name().to_owned(),
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
        connection_info: conn.connection_info().to_owned(),
        db_name,
        database: Arc::new(conn),
        sql_family: SqlFamily::Mysql,
        introspection_connector,
    }
}

pub async fn mysql_8_test_api(db_name: &'static str) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(db_name);
    let url = mysql_8_url(db_name.as_ref());
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();

    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        connection_info: conn.connection_info().to_owned(),
        db_name,
        database: Arc::new(conn),
        sql_family: SqlFamily::Mysql,
        introspection_connector,
    }
}

pub async fn mysql_mariadb_test_api(db_name: &'static str) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(db_name);
    let url = mariadb_url(db_name.as_ref());
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();

    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        db_name,
        connection_info: conn.connection_info().to_owned(),
        database: Arc::new(conn),
        sql_family: SqlFamily::Mysql,
        introspection_connector,
    }
}

pub async fn postgres_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_10_url(db_name),db_name).await
}

pub async fn postgres9_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_9_url(db_name),db_name).await
}

pub async fn postgres11_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_11_url(db_name),db_name).await
}

pub async fn postgres12_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_12_url(db_name),db_name).await
}

pub async fn test_api_helper_for_postgres(url: String,db_name: &'static str) -> TestApi {
    let database = test_setup::create_postgres_database(&url.parse().unwrap()).await.unwrap();
    let connection_info = database.connection_info().to_owned();
    let drop_schema = dbg!(format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", connection_info.schema_name()));
    database.query_raw(&drop_schema, &[]).await.ok();

    let create_schema = dbg!(format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", connection_info.schema_name()));
    database.query_raw(&create_schema, &[]).await.ok();
    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        connection_info,
        db_name,
        database: Arc::new(database),
        sql_family: SqlFamily::Postgres,
        introspection_connector,
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
        connection_info: database.connection_info().to_owned(),
        database: Arc::new(database),
        sql_family: SqlFamily::Sqlite,
        introspection_connector,
    }
}
