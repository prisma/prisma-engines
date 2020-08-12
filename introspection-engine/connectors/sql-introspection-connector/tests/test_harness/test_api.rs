use super::misc_helpers::*;
use datamodel::Datamodel;
use introspection_connector::{DatabaseMetadata, IntrospectionConnector, Version};
use quaint::{
    prelude::{ConnectionInfo, Queryable, SqlFamily},
    single::Quaint,
};
use sql_introspection_connector::SqlIntrospectionConnector;
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::sync::Arc;
use test_setup::*;

pub type TestResult = Result<(), anyhow::Error>;

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

    pub fn database(&self) -> &Arc<dyn Queryable + Send + Sync + 'static> {
        &self.database
    }

    pub async fn describe_schema(&self) -> anyhow::Result<SqlSchema> {
        match &self.connection_info {
            ConnectionInfo::Mssql(_) => todo!("implement MSSQL"),
            ConnectionInfo::Postgres(url) => {
                let sql_schema = sql_schema_describer::postgres::SqlSchemaDescriber::new(Arc::clone(&self.database))
                    .describe(url.schema())
                    .await?;

                Ok(sql_schema)
            }
            ConnectionInfo::Mysql(_url) => {
                let sql_schema = sql_schema_describer::mysql::SqlSchemaDescriber::new(Arc::clone(&self.database))
                    .describe(self.connection_info.schema_name())
                    .await?;

                Ok(sql_schema)
            }
            ConnectionInfo::Sqlite {
                file_path: _,
                db_name: _,
            } => {
                let sql_schema = sql_schema_describer::sqlite::SqlSchemaDescriber::new(Arc::clone(&self.database))
                    .describe(self.connection_info.schema_name())
                    .await?;

                Ok(sql_schema)
            }
        }
    }

    pub async fn introspect(&self) -> String {
        let introspection_result = self
            .introspection_connector
            .introspect(&Datamodel::new(), false)
            .await
            .unwrap();
        datamodel::render_datamodel_to_string(&introspection_result.data_model).expect("Datamodel rendering failed")
    }

    pub async fn re_introspect(&self, data_model_string: &str) -> String {
        let data_model = datamodel::parse_datamodel(data_model_string).unwrap();
        let introspection_result = self
            .introspection_connector
            .introspect(&data_model, true)
            .await
            .unwrap();
        datamodel::render_datamodel_to_string(&introspection_result.data_model).expect("Datamodel rendering failed")
    }

    pub async fn re_introspect_warnings(&self, data_model_string: &str) -> String {
        let data_model = datamodel::parse_datamodel(data_model_string).unwrap();
        let introspection_result = self
            .introspection_connector
            .introspect(&data_model, true)
            .await
            .unwrap();
        serde_json::to_string(&introspection_result.warnings).unwrap()
    }

    pub async fn introspect_version(&self) -> Version {
        let introspection_result = self
            .introspection_connector
            .introspect(&Datamodel::new(), false)
            .await
            .unwrap();
        introspection_result.version
    }

    pub async fn introspection_warnings(&self) -> String {
        let introspection_result = self
            .introspection_connector
            .introspect(&Datamodel::new(), false)
            .await
            .unwrap();
        serde_json::to_string(&introspection_result.warnings).unwrap()
    }

    pub async fn get_metadata(&self) -> DatabaseMetadata {
        self.introspection_connector.get_metadata().await.unwrap()
    }

    pub async fn get_database_description(&self) -> String {
        self.introspection_connector.get_database_description().await.unwrap()
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
                SqlFamily::Mssql => todo!("Greetings from Redmond"),
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

pub async fn mysql_5_6_test_api(db_name: &'static str) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(db_name);
    let url = mysql_5_6_url(db_name.as_ref());
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
    test_api_helper_for_postgres(postgres_10_url(db_name), db_name).await
}

pub async fn postgres9_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_9_url(db_name), db_name).await
}

pub async fn postgres11_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_11_url(db_name), db_name).await
}

pub async fn postgres12_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_12_url(db_name), db_name).await
}

pub async fn postgres13_test_api(db_name: &'static str) -> TestApi {
    test_api_helper_for_postgres(postgres_13_url(db_name), db_name).await
}

pub async fn test_api_helper_for_postgres(url: String, db_name: &'static str) -> TestApi {
    let database = test_setup::create_postgres_database(&url.parse().unwrap())
        .await
        .unwrap();
    let connection_info = database.connection_info().to_owned();
    let drop_schema = dbg!(format!(
        "DROP SCHEMA IF EXISTS \"{}\" CASCADE;",
        connection_info.schema_name()
    ));
    database.query_raw(&drop_schema, &[]).await.ok();

    let create_schema = dbg!(format!(
        "CREATE SCHEMA IF NOT EXISTS \"{}\";",
        connection_info.schema_name()
    ));
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
