pub use super::TestResult;
pub use test_setup::{BitFlags, Capabilities, Tags};

use crate::{BarrelMigrationExecutor, Result};
use datamodel::{Configuration, Datamodel};
use introspection_connector::{DatabaseMetadata, IntrospectionConnector, Version};
use introspection_core::rpc::RpcImpl;
use migration_connector::MigrationConnector;
use quaint::{prelude::SqlFamily, single::Quaint};
use sql_introspection_connector::SqlIntrospectionConnector;
use sql_migration_connector::SqlMigrationConnector;
use sql_schema_describer::SqlSchema;
use std::fmt::Write;
use test_setup::{sqlite_test_url, DatasourceBlock, TestApiArgs};
use tracing::Instrument;

pub struct TestApi {
    api: SqlIntrospectionConnector,
    database: Quaint,
    args: TestApiArgs,
    connection_string: String,
}

impl TestApi {
    pub async fn new(args: TestApiArgs) -> Self {
        let tags = args.tags();
        let connection_string = args.database_url();

        let (database, connection_string): (Quaint, String) = if tags.intersects(Tags::Vitess) {
            let me = SqlMigrationConnector::new(&connection_string, None).await.unwrap();
            me.reset().await.unwrap();

            (
                Quaint::new(&connection_string).await.unwrap(),
                connection_string.to_owned(),
            )
        } else if tags.contains(Tags::Mysql) {
            let (_, cs) = args.create_mysql_database().await;
            (Quaint::new(&cs).await.unwrap(), cs)
        } else if tags.contains(Tags::Postgres) {
            let (_, q, cs) = args.create_postgres_database().await;
            (q, cs)
        } else if tags.contains(Tags::Mssql) {
            test_setup::init_mssql_database(args.database_url(), args.test_function_name())
                .await
                .unwrap()
        } else if tags.contains(Tags::Sqlite) {
            let url = sqlite_test_url(args.test_function_name());
            (Quaint::new(&url).await.unwrap(), url)
        } else {
            unreachable!()
        };

        let api = SqlIntrospectionConnector::new(&connection_string).await.unwrap();

        TestApi {
            api,
            database,
            args,
            connection_string,
        }
    }

    pub async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(self.api.list_databases().await?)
    }

    pub fn database(&self) -> &Quaint {
        &self.database
    }

    pub async fn describe_schema(&self) -> Result<SqlSchema> {
        Ok(self.api.describe().await?)
    }

    pub async fn introspect(&self) -> Result<String> {
        let introspection_result = self.api.introspect(&Datamodel::new()).await?;
        Ok(datamodel::render_datamodel_and_config_to_string(
            &introspection_result.data_model,
            &self.configuration(),
        ))
    }

    pub fn is_cockroach(&self) -> bool {
        self.tags().contains(Tags::Cockroach)
    }

    #[tracing::instrument(skip(self, data_model_string))]
    #[track_caller]
    pub async fn re_introspect(&self, data_model_string: &str) -> Result<String> {
        let config = self.configuration();
        let data_model = parse_datamodel(data_model_string);

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
        let data_model = parse_datamodel(data_model_string);
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
            tags: self.tags(),
        }
    }

    pub fn db_name(&self) -> &str {
        if self.tags().intersects(Tags::Vitess) {
            "test"
        } else {
            self.args.test_function_name()
        }
    }

    pub fn tags(&self) -> BitFlags<Tags> {
        self.args.tags()
    }

    pub fn datasource_block(&self) -> DatasourceBlock<'_> {
        self.args.datasource_block(&self.connection_string, &[])
    }

    pub fn configuration(&self) -> Configuration {
        datamodel::parse_configuration(&self.datasource_block().to_string())
            .unwrap()
            .subject
    }

    #[track_caller]
    pub fn assert_eq_datamodels(&self, expected_without_header: &str, result_with_header: &str) {
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

        write!(out, "{}\n{}", self.datasource_block(), schema).unwrap();

        out
    }
}

#[track_caller]
fn parse_datamodel(dm: &str) -> Datamodel {
    RpcImpl::parse_datamodel(dm).unwrap()
}
