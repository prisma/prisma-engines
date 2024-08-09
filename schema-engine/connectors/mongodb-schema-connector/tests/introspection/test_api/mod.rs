mod utils;

use enumflags2::BitFlags;
pub use expect_test::expect;
use expect_test::Expect;
use itertools::Itertools;
use mongodb::Database;
use mongodb_schema_connector::MongoDbSchemaConnector;
use once_cell::sync::Lazy;
use psl::PreviewFeature;
use schema_connector::{
    CompositeTypeDepth, ConnectorParams, IntrospectionContext, IntrospectionResult, SchemaConnector,
};
use std::{future::Future, path::PathBuf};
use tokio::runtime::Runtime;

pub use utils::*;

pub static RT: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

pub struct TestResult {
    datamodel: String,
    warnings: String,
}

impl TestResult {
    pub fn datamodel(&self) -> &str {
        &self.datamodel
    }

    #[track_caller]
    pub fn expect_warnings(&self, expect: &Expect) {
        expect.assert_eq(&self.warnings);
    }
}

pub struct TestMultiResult {
    datamodels: String,
    warnings: String,
}

impl TestMultiResult {
    pub fn datamodels(&self) -> &str {
        &self.datamodels
    }
}

impl From<IntrospectionResult> for TestResult {
    fn from(res: IntrospectionResult) -> Self {
        Self {
            datamodel: res.datamodels.into_iter().next().unwrap().1,
            warnings: res.warnings.unwrap_or_default(),
        }
    }
}

impl From<IntrospectionResult> for TestMultiResult {
    fn from(res: IntrospectionResult) -> Self {
        let datamodels = res
            .datamodels
            .into_iter()
            .sorted_unstable_by_key(|(file_name, _)| file_name.to_owned())
            .map(|(file_name, dm)| format!("// file: {file_name}\n{dm}"))
            .join("------\n");

        Self {
            datamodels,
            warnings: res.warnings.unwrap_or_default(),
        }
    }
}

pub struct TestApi {
    pub connection_string: String,
    pub database_name: String,
    pub db: Database,
    pub features: BitFlags<PreviewFeature>,
    pub connector: MongoDbSchemaConnector,
}

impl TestApi {
    pub async fn re_introspect_multi(&mut self, datamodels: &[(&str, String)], expectation: expect_test::Expect) {
        let schema = parse_datamodels(datamodels);
        let ctx = IntrospectionContext::new(schema, CompositeTypeDepth::Infinite, None, PathBuf::new());
        let reintrospected = self.connector.introspect(&ctx).await.unwrap();
        let reintrospected = TestMultiResult::from(reintrospected);

        expectation.assert_eq(reintrospected.datamodels());
    }

    pub async fn expect_warnings(&mut self, expectation: &expect_test::Expect) {
        let previous_schema = psl::validate(config_block_string(self.features).into());
        let ctx = IntrospectionContext::new(previous_schema, CompositeTypeDepth::Infinite, None, PathBuf::new());
        let result = self.connector.introspect(&ctx).await.unwrap();
        let result = TestMultiResult::from(result);

        expectation.assert_eq(&result.warnings);
    }
}

pub(super) fn with_database_features<F, U, T>(
    setup: F,
    preview_features: BitFlags<PreviewFeature>,
) -> Result<T, mongodb::error::Error>
where
    F: FnOnce(TestApi) -> U,
    U: Future<Output = mongodb::error::Result<T>>,
{
    let database_name = generate_database_name();
    let connection_string = get_connection_string(&database_name);

    RT.block_on(async move {
        let client = mongodb_client::create(&connection_string).await.unwrap();
        let database = client.database(&database_name);

        let params = ConnectorParams {
            connection_string: connection_string.clone(),
            preview_features,
            shadow_database_connection_string: None,
        };

        let connector = MongoDbSchemaConnector::new(params);

        let api = TestApi {
            connection_string,
            database_name,
            db: database.clone(),
            features: preview_features,
            connector,
        };

        let res = setup(api).await;

        database.drop(None).await.unwrap();

        res
    })
}

pub(super) fn with_database<F, U, T>(setup: F) -> Result<T, mongodb::error::Error>
where
    F: FnMut(TestApi) -> U,
    U: Future<Output = mongodb::error::Result<T>>,
{
    with_database_features(setup, BitFlags::empty())
}

pub(super) fn introspect_features<F, U>(
    composite_type_depth: CompositeTypeDepth,
    preview_features: BitFlags<PreviewFeature>,
    init_database: F,
) -> TestResult
where
    F: FnOnce(Database) -> U,
    U: Future<Output = mongodb::error::Result<()>>,
{
    let datamodel_string = config_block_string(preview_features);
    let validated_schema = psl::parse_schema(datamodel_string).unwrap();
    let ctx = IntrospectionContext::new(validated_schema, composite_type_depth, None, PathBuf::new())
        .without_config_rendering();
    let res = with_database_features(
        |mut api| async move {
            init_database(api.db).await.unwrap();

            let res = api.connector.introspect(&ctx).await.unwrap();

            Ok(res)
        },
        preview_features,
    )
    .unwrap();

    TestResult {
        datamodel: res.datamodels.into_iter().next().unwrap().1,
        warnings: res.warnings.unwrap_or_default(),
    }
}

pub(super) fn introspect_depth<F, U>(composite_type_depth: CompositeTypeDepth, init_database: F) -> TestResult
where
    F: FnOnce(Database) -> U,
    U: Future<Output = mongodb::error::Result<()>>,
{
    let enabled_preview_features = BitFlags::all();
    introspect_features(composite_type_depth, enabled_preview_features, init_database)
}

pub(super) fn introspect<F, U>(init_database: F) -> TestResult
where
    F: FnOnce(Database) -> U,
    U: Future<Output = mongodb::error::Result<()>>,
{
    introspect_depth(CompositeTypeDepth::Infinite, init_database)
}
