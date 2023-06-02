use enumflags2::BitFlags;
use expect_test::Expect;
use mongodb::Database;
use mongodb_schema_connector::MongoDbSchemaConnector;
use names::Generator;
use once_cell::sync::Lazy;
use psl::PreviewFeature;
use schema_connector::{CompositeTypeDepth, ConnectorParams, IntrospectionContext, SchemaConnector};
use std::{future::Future, io::Write};
use tokio::runtime::Runtime;

pub use expect_test::expect;

pub static CONN_STR: Lazy<String> = Lazy::new(|| match std::env::var("TEST_DATABASE_URL") {
    Ok(url) => url,
    Err(_) => {
        let stderr = std::io::stderr();

        let mut sink = stderr.lock();
        sink.write_all(b"Please set TEST_DATABASE_URL env var pointing to a MongoDB instance.")
            .unwrap();
        sink.write_all(b"\n").unwrap();

        std::process::exit(1)
    }
});

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

pub(super) fn introspect_features<F, U>(
    composite_type_depth: CompositeTypeDepth,
    preview_features: BitFlags<PreviewFeature>,
    init_database: F,
) -> TestResult
where
    F: FnOnce(Database) -> U,
    U: Future<Output = mongodb::error::Result<()>>,
{
    let mut names = Generator::default();

    let database_name = names.next().unwrap().replace('-', "");
    let mut connection_string: url::Url = CONN_STR.parse().unwrap();
    connection_string.set_path(&format!(
        "/{}{}",
        database_name,
        connection_string.path().trim_start_matches('/')
    ));
    let connection_string = connection_string.to_string();

    let features = preview_features
        .iter()
        .map(|f| format!("\"{f}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let datamodel_string = indoc::formatdoc!(
        r#"
            datasource db {{
              provider = "mongodb"
              url      = "{}"
            }}

            generator js {{
              provider        = "prisma-client-js"
              previewFeatures = [{}]
            }}
        "#,
        connection_string,
        features,
    );

    let validated_schema = psl::parse_schema(datamodel_string).unwrap();
    let mut ctx = IntrospectionContext::new(validated_schema, composite_type_depth, None);
    ctx.render_config = false;

    RT.block_on(async move {
        let client = mongodb_client::create(&connection_string).await.unwrap();
        let database = client.database(&database_name);

        let params = ConnectorParams {
            connection_string,
            preview_features,
            shadow_database_connection_string: None,
        };

        let mut connector = MongoDbSchemaConnector::new(params);

        if init_database(database.clone()).await.is_err() {
            database.drop(None).await.unwrap();
        }

        let res = connector.introspect(&ctx).await;
        database.drop(None).await.unwrap();

        let res = res.unwrap();

        TestResult {
            datamodel: res.data_model,
            warnings: res.warnings.unwrap_or_default(),
        }
    })
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
