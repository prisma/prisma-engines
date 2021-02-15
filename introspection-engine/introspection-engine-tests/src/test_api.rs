use crate::BarrelMigrationExecutor;
use datamodel::{Configuration, Datamodel};
use enumflags2::BitFlags;
use eyre::{Context, Report, Result};
use introspection_connector::{DatabaseMetadata, IntrospectionConnector, Version};
use introspection_core::rpc::RpcImpl;
use quaint::{
    prelude::{ConnectionInfo, SqlFamily},
    single::Quaint,
};
use sql_introspection_connector::SqlIntrospectionConnector;
use sql_schema_describer::{mssql, mysql, postgres, sqlite, SqlSchema, SqlSchemaDescriberBackend};
use test_setup::{connectors::Tags, *};
use tracing::Instrument;

pub struct TestApi {
    api: SqlIntrospectionConnector,
    args: TestApiArgs,
    connection_string: String,
    db_name: &'static str,
    database: Quaint,
}

impl TestApi {
    pub async fn new(args: TestApiArgs) -> Self {
        let tags = args.connector_tags;

        let db_name = if args.connector_tags.contains(Tags::Mysql) {
            test_setup::mysql_safe_identifier(args.test_function_name)
        } else {
            args.test_function_name
        };

        let connection_string = (args.url_fn)(db_name);

        let database = if tags.contains(Tags::Mysql) {
            create_mysql_database(&connection_string.parse().unwrap())
                .await
                .unwrap()
        } else if tags.contains(Tags::Postgres) {
            create_postgres_database(&connection_string.parse().unwrap())
                .await
                .unwrap()
        } else if tags.contains(Tags::Mssql) {
            let conn = Quaint::new(&connection_string).await.unwrap();

            test_setup::connectors::mssql::reset_schema(&conn, db_name)
                .await
                .unwrap();

            conn
        } else if tags.contains(Tags::Sqlite) {
            Quaint::new(&connection_string).await.unwrap()
        } else {
            unreachable!()
        };

        let api = SqlIntrospectionConnector::new(&connection_string).await.unwrap();

        TestApi {
            api,
            args,
            connection_string,
            database,
            db_name,
        }
    }

    pub async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(self.api.list_databases().await?)
    }

    pub fn database(&self) -> &Quaint {
        &self.database
    }

    pub async fn describe_schema(&self) -> Result<SqlSchema> {
        match &self.database.connection_info() {
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
                    .describe(self.database.connection_info().schema_name())
                    .await?;

                Ok(sql_schema)
            }
            ConnectionInfo::Sqlite {
                file_path: _,
                db_name: _,
            }
            | ConnectionInfo::InMemorySqlite { .. } => {
                let sql_schema = sqlite::SqlSchemaDescriber::new(self.database.clone())
                    .describe(self.database.connection_info().schema_name())
                    .await?;

                Ok(sql_schema)
            }
        }
    }

    pub async fn introspect(&self) -> Result<String> {
        let introspection_result = self.api.introspect(&Datamodel::new()).await?;
        Ok(datamodel::render_datamodel_and_config_to_string(
            &introspection_result.data_model,
            &self.configuration(),
        ))
    }

    #[tracing::instrument(skip(self, data_model_string))]
    pub async fn re_introspect(&self, data_model_string: &str) -> Result<String> {
        let config = parse_configuration(data_model_string).context("parsing configuration")?;
        let data_model = parse_datamodel(data_model_string).context("parsing datamodel")?;

        let introspection_result = self
            .api
            .introspect(&data_model)
            .instrument(tracing::info_span!("introspect"))
            .await?;

        let rendering_span = tracing::info_span!("render_datamodel after introspection");
        let _span = rendering_span.enter();
        let dm = datamodel::render_datamodel_and_config_to_string(&introspection_result.data_model, &config);

        Ok(dm)
    }

    pub async fn re_introspect_warnings(&self, data_model_string: &str) -> Result<String> {
        let data_model = parse_datamodel(data_model_string)?;
        let introspection_result = self.api.introspect(&data_model).await?;

        Ok(serde_json::to_string(&introspection_result.warnings)?)
    }

    pub async fn introspect_version(&self) -> Result<Version> {
        let introspection_result = self.api.introspect(&Datamodel::new()).await?;

        Ok(introspection_result.version)
    }

    pub async fn introspection_warnings(&self) -> Result<String> {
        let introspection_result = self.api.introspect(&Datamodel::new()).await?;

        Ok(serde_json::to_string(&introspection_result.warnings)?)
    }

    pub async fn get_metadata(&self) -> Result<DatabaseMetadata> {
        Ok(self.api.get_metadata().await?)
    }

    pub async fn get_database_description(&self) -> Result<String> {
        Ok(self.api.get_database_description().await?)
    }

    pub async fn get_database_version(&self) -> Result<String> {
        Ok(self.api.get_database_version().await?)
    }

    pub fn sql_family(&self) -> SqlFamily {
        self.database.connection_info().sql_family()
    }

    pub fn schema_name(&self) -> &str {
        self.database.connection_info().schema_name()
    }

    pub fn barrel(&self) -> BarrelMigrationExecutor {
        BarrelMigrationExecutor {
            schema_name: self.schema_name().to_owned(),
            database: self.database.clone(),
            sql_variant: match self.sql_family() {
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
    pub fn tags(&self) -> BitFlags<Tags> {
        self.args.connector_tags
    }

    pub fn datasource_block(&self) -> String {
        self.args.datasource_block(&self.connection_string)
    }

    pub fn configuration(&self) -> Configuration {
        datamodel::parse_configuration(&self.datasource_block())
            .unwrap()
            .subject
    }

    pub fn assert_eq_datamodels(&self, expected_without_header: &str, result_with_header: &str) -> () {
        let parsed_expected = datamodel::parse_datamodel(&self.dm_with_sources(expected_without_header))
            .unwrap()
            .subject;
        let parsed_result = datamodel::parse_datamodel(result_with_header).unwrap().subject;

        let reformatted_expected =
            datamodel::render_datamodel_and_config_to_string(&parsed_expected, &self.configuration());
        let reformatted_result =
            datamodel::render_datamodel_and_config_to_string(&parsed_result, &self.configuration());

        pretty_assertions::assert_eq!(reformatted_expected, reformatted_result);
    }

    pub fn dm_with_sources(&self, schema: &str) -> String {
        let mut out = String::with_capacity(320 + schema.len());

        out.push_str(&self.datasource_block());
        out.push_str(schema);

        out
    }
}

fn parse_datamodel(dm: &str) -> Result<Datamodel> {
    match RpcImpl::parse_datamodel(dm) {
        Ok(dm) => Ok(dm),
        Err(e) => Err(Report::msg(serde_json::to_string_pretty(&e.data).unwrap())),
    }
}

fn parse_configuration(dm: &str) -> Result<Configuration> {
    match datamodel::parse_configuration(dm) {
        Ok(dm) => Ok(dm.subject),
        Err(e) => Err(Report::msg(e.to_pretty_string("schema.prisma", dm))),
    }
}
