use super::misc_helpers::*;
use datamodel::configuration::preview_features::PreviewFeatures;
use datamodel::Datamodel;
use introspection_connector::{DatabaseMetadata, IntrospectionConnector, Version};
use quaint::{
    prelude::{ConnectionInfo, SqlFamily},
    single::Quaint,
};
use sql_introspection_connector::SqlIntrospectionConnector;
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use test_setup::*;

pub type TestResult = Result<(), anyhow::Error>;

pub struct TestApi {
    db_name: &'static str,
    connection_info: quaint::prelude::ConnectionInfo,
    sql_family: SqlFamily,
    database: Quaint,
    introspection_connector: SqlIntrospectionConnector,
}

impl TestApi {
    pub async fn list_databases(&self) -> Vec<String> {
        self.introspection_connector.list_databases().await.unwrap()
    }

    pub fn database(&self) -> &Quaint {
        &self.database
    }

    pub async fn describe_schema(&self) -> anyhow::Result<SqlSchema> {
        match &self.connection_info {
            ConnectionInfo::Mssql(_) => todo!("implement MSSQL"),
            ConnectionInfo::Postgres(url) => {
                let sql_schema = sql_schema_describer::postgres::SqlSchemaDescriber::new(self.database.clone())
                    .describe(url.schema())
                    .await?;

                Ok(sql_schema)
            }
            ConnectionInfo::Mysql(_url) => {
                let sql_schema = sql_schema_describer::mysql::SqlSchemaDescriber::new(self.database.clone())
                    .describe(self.connection_info.schema_name())
                    .await?;

                Ok(sql_schema)
            }
            ConnectionInfo::Sqlite {
                file_path: _,
                db_name: _,
            } => {
                let sql_schema = sql_schema_describer::sqlite::SqlSchemaDescriber::new(self.database.clone())
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
        let config = datamodel::parse_configuration(data_model_string).unwrap();

        let native_types = config.datasources.first().has_preview_feature("nativeTypes");

        let introspection_result = self
            .introspection_connector
            .introspect(&data_model, native_types)
            .await
            .unwrap();
        datamodel::render_datamodel_and_config_to_string(&introspection_result.data_model, &config)
            .expect("Datamodel rendering failed")
    }

    pub async fn re_introspect_warnings(&self, data_model_string: &str) -> String {
        let data_model = datamodel::parse_datamodel(data_model_string).unwrap();
        let introspection_result = self
            .introspection_connector
            .introspect(&data_model, false)
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

    pub async fn get_database_version(&self) -> String {
        self.introspection_connector.get_database_version().await.unwrap()
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
            database: self.database.clone(),
            sql_variant: match self.sql_family {
                SqlFamily::Mysql => barrel::SqlVariant::Mysql,
                SqlFamily::Postgres => barrel::SqlVariant::Pg,
                SqlFamily::Sqlite => barrel::SqlVariant::Sqlite,
                SqlFamily::Mssql => barrel::SqlVariant::Mssql,
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
        database: conn,
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
        database: conn,
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
        database: conn,
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
        database: conn,
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
    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        connection_info,
        db_name,
        database,
        sql_family: SqlFamily::Postgres,
        introspection_connector,
    }
}

pub async fn sqlite_test_api(db_name: &'static str) -> TestApi {
    sqlite_test_file(db_name);
    let connection_string = sqlite_test_url(db_name);
    let database = Quaint::new(&connection_string).await.unwrap();
    let introspection_connector = SqlIntrospectionConnector::new(&connection_string).await.unwrap();

    TestApi {
        db_name,
        connection_info: database.connection_info().to_owned(),
        database,
        sql_family: SqlFamily::Sqlite,
        introspection_connector,
    }
}

pub async fn mssql_2017_test_api(schema: &'static str) -> TestApi {
    mssql_test_api(mssql_2017_url("master"), schema).await
}

pub async fn mssql_2019_test_api(schema: &'static str) -> TestApi {
    mssql_test_api(mssql_2019_url("master"), schema).await
}

pub async fn mssql_test_api(connection_string: String, schema: &'static str) -> TestApi {
    use test_setup::connectors::mssql;

    let connection_string = format!("{};schema={}", connection_string, schema);
    let database = Quaint::new(&connection_string).await.unwrap();
    let connection_info = database.connection_info().to_owned();

    mssql::reset_schema(&database, schema).await.unwrap();

    let introspection_connector = SqlIntrospectionConnector::new(&connection_string).await.unwrap();

    TestApi {
        db_name: schema,
        connection_info,
        database,
        sql_family: SqlFamily::Mssql,
        introspection_connector,
    }
}
