use enumflags2::BitFlags;
use introspection_connector::{CompositeTypeDepth, IntrospectionConnector, IntrospectionContext, Warning};
use mongodb::Database;
use mongodb_introspection_connector::MongoDbIntrospectionConnector;
use names::Generator;
use once_cell::sync::Lazy;
use psl::PreviewFeature;
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
    warnings: Vec<Warning>,
}

impl TestResult {
    pub fn datamodel(&self) -> &str {
        &self.datamodel
    }

    #[track_caller]
    pub fn assert_warning_code(&self, code: u32) {
        assert!(self.warnings.iter().any(|w| w.code == code), "{:#?}", self.warnings)
    }

    #[track_caller]
    pub fn assert_warning(&self, warning: &str) {
        assert!(
            self.warnings.iter().any(|w| w.message == warning),
            "{:#?}",
            self.warnings
        )
    }

    #[track_caller]
    pub fn assert_no_warnings(&self) {
        assert!(self.warnings.is_empty(), "{:#?}", self.warnings)
    }

    #[track_caller]
    pub fn assert_warning_affected(&self, affected: &serde_json::Value) {
        assert!(&self.warnings[0].affected == affected, "{:#?}", self.warnings);
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
        .map(|f| format!("\"{}\"", f))
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
    let mut ctx = IntrospectionContext::new(validated_schema, None, composite_type_depth);
    ctx.render_config = false;

    RT.block_on(async move {
        let client = mongodb_client::create(&connection_string).await.unwrap();
        let database = client.database(&database_name);
        let connector = MongoDbIntrospectionConnector::new(&connection_string).await.unwrap();

        if init_database(database.clone()).await.is_err() {
            database.drop(None).await.unwrap();
        }

        let res = connector.introspect(&ctx).await;
        database.drop(None).await.unwrap();

        let res = res.unwrap();

        TestResult {
            datamodel: res.data_model,
            warnings: res.warnings,
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

pub(super) fn get_database_description<F, U>(init_database: F) -> String
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

    RT.block_on(async move {
        let client = mongodb_client::create(&connection_string).await.unwrap();
        let database = client.database(&database_name);
        let connector = MongoDbIntrospectionConnector::new(&connection_string).await.unwrap();

        if init_database(database.clone()).await.is_err() {
            database.drop(None).await.unwrap();
        }

        let res = connector.get_database_description().await;
        database.drop(None).await.unwrap();

        res.unwrap()
    })
}

pub(super) fn get_database_version<F, U>(init_database: F) -> String
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

    RT.block_on(async move {
        let client = mongodb_client::create(&connection_string).await.unwrap();
        let database = client.database(&database_name);
        let connector = MongoDbIntrospectionConnector::new(&connection_string).await.unwrap();

        if init_database(database.clone()).await.is_err() {
            database.drop(None).await.unwrap();
        }

        let res = connector.get_database_version().await;
        database.drop(None).await.unwrap();

        res.unwrap()
    })
}
