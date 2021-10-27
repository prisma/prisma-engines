pub use super::TestResult;
pub use expect_test::expect;
pub use test_macros::test_connector;
pub use test_setup::{BitFlags, Capabilities, Tags};

use crate::{BarrelMigrationExecutor, Result};
use datamodel::common::preview_features::PreviewFeature;
use datamodel::{Configuration, Datamodel};
use introspection_connector::{
    CompositeTypeDepth, ConnectorResult, DatabaseMetadata, IntrospectionConnector, IntrospectionContext,
    IntrospectionResult, Version,
};
use migration_connector::MigrationConnector;
use quaint::{
    prelude::{Queryable, SqlFamily},
    single::Quaint,
};
use sql_introspection_connector::SqlIntrospectionConnector;
use sql_migration_connector::SqlMigrationConnector;
use std::fmt::Write;
use test_setup::{sqlite_test_url, DatasourceBlock, TestApiArgs};
use tracing::Instrument;

pub struct TestApi {
    api: SqlIntrospectionConnector,
    database: Quaint,
    args: TestApiArgs,
    connection_string: String,
    preview_features: BitFlags<PreviewFeature>,
}

impl TestApi {
    pub async fn new(args: TestApiArgs) -> Self {
        let tags = args.tags();
        let connection_string = args.database_url();

        let preview_features = args
            .preview_features()
            .iter()
            .flat_map(|f| PreviewFeature::parse_opt(f))
            .collect();

        let (database, connection_string): (Quaint, String) = if tags.intersects(Tags::Vitess) {
            let me = SqlMigrationConnector::new(connection_string.to_owned(), preview_features, None).unwrap();

            me.reset().await.unwrap();

            (
                Quaint::new(connection_string).await.unwrap(),
                connection_string.to_owned(),
            )
        } else if tags.contains(Tags::Mysql) {
            let (_, cs) = args.create_mysql_database().await;
            (Quaint::new(&cs).await.unwrap(), cs)
        } else if tags.contains(Tags::Postgres) {
            let (_, q, cs) = args.create_postgres_database().await;
            (q, cs)
        } else if tags.contains(Tags::Mssql) {
            args.create_mssql_database().await
        } else if tags.contains(Tags::Sqlite) {
            let url = sqlite_test_url(args.test_function_name());
            (Quaint::new(&url).await.unwrap(), url)
        } else {
            unreachable!()
        };

        let api = SqlIntrospectionConnector::new(&connection_string, preview_features)
            .await
            .unwrap();

        TestApi {
            api,
            database,
            args,
            connection_string,
            preview_features,
        }
    }

    pub async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(self.api.list_databases().await?)
    }

    pub fn database(&self) -> &Quaint {
        &self.database
    }

    pub async fn introspect(&self) -> Result<String> {
        let introspection_result = self.test_introspect_internal(Datamodel::new()).await?;

        Ok(datamodel::render_datamodel_and_config_to_string(
            &introspection_result.data_model,
            &self.configuration(),
        ))
    }

    pub async fn introspect_dml(&self) -> Result<String> {
        let introspection_result = self.test_introspect_internal(Datamodel::new()).await?;

        Ok(datamodel::render_datamodel_to_string(
            &introspection_result.data_model,
            Some(&self.configuration()),
        ))
    }

    pub fn is_cockroach(&self) -> bool {
        self.tags().contains(Tags::Cockroach)
    }

    pub fn is_mysql8(&self) -> bool {
        self.tags().contains(Tags::Mysql8)
    }

    /// Returns true only when testing on vitess.
    pub fn is_vitess(&self) -> bool {
        self.tags().contains(Tags::Vitess)
    }

    pub fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.preview_features
    }

    #[tracing::instrument(skip(self))]
    #[track_caller]
    async fn test_introspect_internal(&self, data_model: Datamodel) -> ConnectorResult<IntrospectionResult> {
        let config = self.configuration();

        let ctx = IntrospectionContext {
            preview_features: self.preview_features(),
            source: config.datasources.into_iter().next().unwrap(),
            composite_type_depth: CompositeTypeDepth::Infinite,
        };

        self.api
            .introspect(&data_model, ctx)
            .instrument(tracing::info_span!("introspect"))
            .await
    }

    #[tracing::instrument(skip(self, data_model_string))]
    #[track_caller]
    pub async fn re_introspect(&self, data_model_string: &str) -> Result<String> {
        let config = self.configuration();
        let data_model = parse_datamodel(data_model_string);
        let introspection_result = self.test_introspect_internal(data_model).await?;

        let rendering_span = tracing::info_span!("render_datamodel after introspection");
        let _span = rendering_span.enter();
        let dm = datamodel::render_datamodel_and_config_to_string(&introspection_result.data_model, &config);

        Ok(dm)
    }

    #[tracing::instrument(skip(self, data_model_string))]
    #[track_caller]
    pub async fn re_introspect_dml(&self, data_model_string: &str) -> Result<String> {
        let config = self.configuration();
        let data_model = parse_datamodel(data_model_string);
        let introspection_result = self.test_introspect_internal(data_model).await?;

        let rendering_span = tracing::info_span!("render_datamodel after introspection");
        let _span = rendering_span.enter();
        let dm = datamodel::render_datamodel_to_string(&introspection_result.data_model, Some(&config));

        Ok(dm)
    }

    pub async fn re_introspect_warnings(&self, data_model_string: &str) -> Result<String> {
        let data_model = parse_datamodel(data_model_string);
        let introspection_result = self.test_introspect_internal(data_model).await?;

        Ok(serde_json::to_string(&introspection_result.warnings)?)
    }

    pub async fn introspect_version(&self) -> Result<Version> {
        let introspection_result = self.test_introspect_internal(Datamodel::new()).await?;

        Ok(introspection_result.version)
    }

    pub async fn introspection_warnings(&self) -> Result<String> {
        let introspection_result = self.test_introspect_internal(Datamodel::new()).await?;

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
        let no_foreign_keys =
            self.is_vitess() && self.preview_features().contains(PreviewFeature::ReferentialIntegrity);

        if no_foreign_keys {
            self.args
                .datasource_block(&self.connection_string, &[("referentialIntegrity", r#""prisma""#)])
        } else {
            self.args.datasource_block(&self.connection_string, &[])
        }
    }

    pub fn configuration(&self) -> Configuration {
        datamodel::parse_configuration(&format!(
            "{}\n{}",
            &self.datasource_block().to_string(),
            &self.generator_block()
        ))
        .unwrap()
        .subject
    }

    #[track_caller]
    pub fn assert_eq_datamodels(&self, expected_without_header: &str, result_with_header: &str) {
        let expected_with_source = self.dm_with_sources(expected_without_header);
        let expected_with_generator = self.dm_with_generator_and_preview_flags(&expected_with_source);

        let parsed_expected = datamodel::parse_datamodel(&expected_with_generator)
            .map_err(|err| err.to_pretty_string("schema.prisma", &expected_with_generator))
            .unwrap()
            .subject;

        let parsed_result = datamodel::parse_datamodel(result_with_header).unwrap().subject;

        let reformatted_expected =
            datamodel::render_datamodel_and_config_to_string(&parsed_expected, &self.configuration());
        let reformatted_result =
            datamodel::render_datamodel_and_config_to_string(&parsed_result, &self.configuration());

        println!("{}", reformatted_expected);
        println!("{}", reformatted_result);

        pretty_assertions::assert_eq!(reformatted_expected, reformatted_result);
    }

    pub fn dm_with_sources(&self, schema: &str) -> String {
        let mut out = String::with_capacity(320 + schema.len());

        write!(out, "{}\n{}", self.datasource_block(), schema).unwrap();

        out
    }

    pub fn dm_with_generator_and_preview_flags(&self, schema: &str) -> String {
        let mut out = String::with_capacity(320 + schema.len());

        write!(out, "{}\n{}", self.generator_block(), schema).unwrap();

        out
    }

    fn generator_block(&self) -> String {
        let preview_features: Vec<String> = self
            .preview_features()
            .iter()
            .map(|pf| format!(r#""{}""#, pf))
            .collect();

        let preview_feature_string = if preview_features.is_empty() {
            "".to_string()
        } else {
            format!("\npreviewFeatures = [{}]", preview_features.join(", "))
        };

        let generator_block = format!(
            r#"generator client {{
                 provider = "prisma-client-js"{}
               }}"#,
            preview_feature_string
        );
        generator_block
    }

    #[track_caller]
    pub async fn raw_cmd(&self, query: &str) {
        self.api.quaint().raw_cmd(query).await.unwrap()
    }
}

#[track_caller]
fn parse_datamodel(dm: &str) -> Datamodel {
    datamodel::parse_datamodel_or_pretty_error(dm, "schema.prisma")
        .unwrap()
        .subject
}
