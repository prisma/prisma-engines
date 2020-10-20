use crate::BarrelMigrationExecutor;
use datamodel::Configuration;
use datamodel::{preview_features::PreviewFeatures, Datamodel};
use enumflags2::BitFlags;
use eyre::{Report, Result};
use introspection_connector::{DatabaseMetadata, IntrospectionConnector, Version};
use quaint::{
    prelude::{ConnectionInfo, SqlFamily},
    single::Quaint,
};
use sql_introspection_connector::SqlIntrospectionConnector;
use sql_schema_describer::{mssql, mysql, postgres, sqlite, SqlSchema, SqlSchemaDescriberBackend};
use test_setup::connectors::Tags;
use test_setup::*;

pub struct TestApi {
    db_name: &'static str,
    connection_info: ConnectionInfo,
    sql_family: SqlFamily,
    database: Quaint,
    introspection_connector: SqlIntrospectionConnector,
    pub tags: BitFlags<Tags>,
}

impl TestApi {
    pub async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(self.introspection_connector.list_databases().await?)
    }

    pub fn database(&self) -> &Quaint {
        &self.database
    }

    pub async fn describe_schema(&self) -> Result<SqlSchema> {
        match &self.connection_info {
            ConnectionInfo::Mssql(url) => {
                let sql_schema = mssql::SqlSchemaDescriber::new(self.database.clone())
                    .describe(url.schema())
                    .await?;

                Ok(sql_schema)
            }
            ConnectionInfo::Postgres(url) => {
                let sql_schema = postgres::SqlSchemaDescriber::new(self.database.clone())
                    .describe(url.schema())
                    .await?;

                Ok(sql_schema)
            }
            ConnectionInfo::Mysql(_url) => {
                let sql_schema = mysql::SqlSchemaDescriber::new(self.database.clone())
                    .describe(self.connection_info.schema_name())
                    .await?;

                Ok(sql_schema)
            }
            ConnectionInfo::Sqlite {
                file_path: _,
                db_name: _,
            } => {
                let sql_schema = sqlite::SqlSchemaDescriber::new(self.database.clone())
                    .describe(self.connection_info.schema_name())
                    .await?;

                Ok(sql_schema)
            }
        }
    }

    pub async fn introspect(&self) -> Result<String> {
        let introspection_result = self
            .introspection_connector
            .introspect(&Datamodel::new(), false)
            .await?;

        Ok(render_datamodel_to_string(&introspection_result.data_model)?)
    }

    pub async fn re_introspect(&self, data_model_string: &str) -> Result<String> {
        let data_model = parse_datamodel(data_model_string)?;
        let config = parse_configuration(data_model_string)?;
        let native_types = config.generators.iter().any(|g| g.has_preview_feature("nativeTypes"));

        let introspection_result = self
            .introspection_connector
            .introspect(&data_model, native_types)
            .await?;

        let dm = render_datamodel_and_config_to_string(&introspection_result.data_model, &config)?;

        Ok(dm)
    }

    pub async fn re_introspect_warnings(&self, data_model_string: &str) -> Result<String> {
        let data_model = parse_datamodel(data_model_string)?;
        let introspection_result = self.introspection_connector.introspect(&data_model, false).await?;

        Ok(serde_json::to_string(&introspection_result.warnings)?)
    }

    pub async fn introspect_version(&self) -> Result<Version> {
        let introspection_result = self
            .introspection_connector
            .introspect(&Datamodel::new(), false)
            .await?;

        Ok(introspection_result.version)
    }

    pub async fn introspection_warnings(&self) -> Result<String> {
        let introspection_result = self
            .introspection_connector
            .introspect(&Datamodel::new(), false)
            .await?;

        Ok(serde_json::to_string(&introspection_result.warnings)?)
    }

    pub async fn get_metadata(&self) -> Result<DatabaseMetadata> {
        Ok(self.introspection_connector.get_metadata().await?)
    }

    pub async fn get_database_description(&self) -> Result<String> {
        Ok(self.introspection_connector.get_database_description().await?)
    }

    pub async fn get_database_version(&self) -> Result<String> {
        Ok(self.introspection_connector.get_database_version().await?)
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
        self.db_name
    }
}

fn parse_datamodel(dm: &str) -> Result<Datamodel> {
    match datamodel::parse_datamodel(dm) {
        Ok(dm) => Ok(dm.subject),
        Err(e) => Err(Report::msg(e.to_pretty_string("schema.prisma", dm))),
    }
}

fn render_datamodel_to_string(dm: &Datamodel) -> Result<String> {
    match datamodel::render_datamodel_to_string(dm) {
        Ok(dm) => Ok(dm),
        Err(_) => Err(Report::msg("Could not render datamodel to a string.")),
    }
}

fn parse_configuration(dm: &str) -> Result<Configuration> {
    match datamodel::parse_configuration(dm) {
        Ok(dm) => Ok(dm.subject),
        Err(e) => Err(Report::msg(e.to_pretty_string("schema.prisma", dm))),
    }
}

pub fn render_datamodel_and_config_to_string(datamodel: &Datamodel, config: &Configuration) -> Result<String> {
    match datamodel::render_datamodel_and_config_to_string(datamodel, config) {
        Ok(dm) => Ok(dm),
        Err(_) => Err(Report::msg("Could not render datamodel and configuration to a string.")),
    }
}

pub async fn mysql_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(args.test_function_name);
    let url = mysql_url(db_name);
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();
    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        connection_info: conn.connection_info().to_owned(),
        database: conn,
        sql_family: SqlFamily::Mysql,
        introspection_connector,
        db_name,
        tags: args.test_tag,
    }
}

pub async fn mysql_8_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(args.test_function_name);
    let url = mysql_8_url(db_name);
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();

    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        connection_info: conn.connection_info().to_owned(),
        db_name,
        tags: args.test_tag,
        database: conn,
        sql_family: SqlFamily::Mysql,
        introspection_connector,
    }
}

pub async fn mysql_5_6_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(args.test_function_name);
    let url = mysql_5_6_url(db_name);
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();

    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        connection_info: conn.connection_info().to_owned(),
        db_name,
        tags: args.test_tag,
        database: conn,
        sql_family: SqlFamily::Mysql,
        introspection_connector,
    }
}

pub async fn mysql_mariadb_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = test_setup::mysql_safe_identifier(args.test_function_name);
    let url = mariadb_url(db_name);
    let conn = create_mysql_database(&url.parse().unwrap()).await.unwrap();

    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        db_name,
        tags: args.test_tag,
        connection_info: conn.connection_info().to_owned(),
        database: conn,
        sql_family: SqlFamily::Mysql,
        introspection_connector,
    }
}

pub async fn postgres_test_api(args: TestAPIArgs) -> TestApi {
    test_api_helper_for_postgres(postgres_10_url(args.test_function_name), args).await
}

pub async fn postgres9_test_api(args: TestAPIArgs) -> TestApi {
    test_api_helper_for_postgres(postgres_9_url(args.test_function_name), args).await
}

pub async fn postgres11_test_api(args: TestAPIArgs) -> TestApi {
    test_api_helper_for_postgres(postgres_11_url(args.test_function_name), args).await
}

pub async fn postgres12_test_api(args: TestAPIArgs) -> TestApi {
    test_api_helper_for_postgres(postgres_12_url(args.test_function_name), args).await
}

pub async fn postgres13_test_api(args: TestAPIArgs) -> TestApi {
    test_api_helper_for_postgres(postgres_13_url(args.test_function_name), args).await
}

pub async fn test_api_helper_for_postgres(url: String, args: TestAPIArgs) -> TestApi {
    let database = test_setup::create_postgres_database(&url.parse().unwrap())
        .await
        .unwrap();
    let connection_info = database.connection_info().to_owned();
    let introspection_connector = SqlIntrospectionConnector::new(&url).await.unwrap();

    TestApi {
        connection_info,
        db_name: args.test_function_name,
        tags: args.test_tag,
        database,
        sql_family: SqlFamily::Postgres,
        introspection_connector,
    }
}

pub async fn sqlite_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    sqlite_test_file(db_name);
    let connection_string = sqlite_test_url(db_name);
    let database = Quaint::new(&connection_string).await.unwrap();
    let introspection_connector = SqlIntrospectionConnector::new(&connection_string).await.unwrap();

    TestApi {
        db_name,
        tags: args.test_tag,
        connection_info: database.connection_info().to_owned(),
        database,
        sql_family: SqlFamily::Sqlite,
        introspection_connector,
    }
}

pub async fn mssql_2017_test_api(args: TestAPIArgs) -> TestApi {
    mssql_test_api(mssql_2017_url("master"), args).await
}

pub async fn mssql_2019_test_api(args: TestAPIArgs) -> TestApi {
    mssql_test_api(mssql_2019_url("master"), args).await
}

pub async fn mssql_test_api(connection_string: String, args: TestAPIArgs) -> TestApi {
    use test_setup::connectors::mssql;
    let schema = args.test_function_name;
    let connection_string = format!("{};schema={}", connection_string, schema);
    let database = Quaint::new(&connection_string).await.unwrap();
    let connection_info = database.connection_info().to_owned();

    mssql::reset_schema(&database, schema).await.unwrap();

    let introspection_connector = SqlIntrospectionConnector::new(&connection_string).await.unwrap();

    TestApi {
        db_name: schema,
        tags: args.test_tag,
        connection_info,
        database,
        sql_family: SqlFamily::Mssql,
        introspection_connector,
    }
}
