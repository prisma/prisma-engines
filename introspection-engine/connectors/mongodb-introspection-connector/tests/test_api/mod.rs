use datamodel::common::preview_features::PreviewFeature;
use introspection_connector::{CompositeTypeDepth, IntrospectionConnector, IntrospectionContext, Warning};
use mongodb::{Client, Database};
use mongodb_introspection_connector::MongoDbIntrospectionConnector;
use names::Generator;
use once_cell::sync::Lazy;
use std::{future::Future, io::Write};
use tokio::runtime::Runtime;

pub use bson::doc;
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
    pub fn assert_warning(&self, warning: &str) {
        dbg!(&self.warnings);
        assert!(self.warnings.iter().any(|w| w.message == warning))
    }
}

pub(super) fn introspect_depth<F, U>(composite_type_depth: CompositeTypeDepth, init_database: F) -> TestResult
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

    let datamodel_string = indoc::formatdoc!(
        r#"
            datasource db {{
              provider = "mongodb"
              url      = "{}"
            }}

            generator js {{
              provider        = "prisma-client-js"
              previewFeatures = ["mongodb"]
            }}
        "#,
        connection_string
    );

    let mut config = datamodel::parse_configuration(&datamodel_string).unwrap();
    let datamodel = datamodel::parse_datamodel(&datamodel_string).unwrap();

    let ctx = IntrospectionContext {
        source: config.subject.datasources.pop().unwrap(),
        composite_type_depth,
        preview_features: PreviewFeature::MongoDb.into(),
    };

    RT.block_on(async move {
        let client = Client::with_uri_str(&connection_string).await.unwrap();
        let database = client.database(&database_name);
        let connector = MongoDbIntrospectionConnector::new(&connection_string).await.unwrap();

        if init_database(database.clone()).await.is_err() {
            database.drop(None).await.unwrap();
        }

        let res = connector.introspect(&datamodel.subject, ctx).await;
        database.drop(None).await.unwrap();

        let res = res.unwrap();
        let config = datamodel::parse_configuration(&datamodel_string).unwrap().subject;

        TestResult {
            datamodel: datamodel::render_datamodel_to_string(&res.data_model, Some(&config)),
            warnings: res.warnings,
        }
    })
}

pub(super) fn introspect<F, U>(init_database: F) -> TestResult
where
    F: FnOnce(Database) -> U,
    U: Future<Output = mongodb::error::Result<()>>,
{
    introspect_depth(CompositeTypeDepth::Infinite, init_database)
}
