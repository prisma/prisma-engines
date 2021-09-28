use datamodel::common::preview_features::PreviewFeature;
use introspection_connector::{IntrospectionConnector, IntrospectionContext, Warning};
use mongodb::{Client, Database};
use mongodb_introspection_connector::MongoDbIntrospectionConnector;
use names::Generator;
use once_cell::sync::Lazy;
use std::future::Future;
use tokio::runtime::Runtime;

static CONN_STR: Lazy<String> = Lazy::new(|| {
    std::env::var("TEST_DATABASE_URL").expect("Please set TEST_DATABASE_URL env var pointing to the MongoDB instance.")
});

static RT: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

pub struct TestResult {
    datamodel: String,
    #[allow(dead_code)] // clippy is wrong
    warnings: Vec<Warning>,
}

impl TestResult {
    pub fn datamodel(&self) -> &str {
        &self.datamodel
    }

    #[track_caller]
    #[allow(dead_code)] // clippy is wrong
    pub fn assert_warning(&self, warning: &str) {
        dbg!(&self.warnings);
        assert!(self.warnings.iter().any(|w| w.message == warning))
    }
}

pub(super) fn introspect<F, U>(init_database: F) -> TestResult
where
    F: FnOnce(Database) -> U,
    U: Future<Output = mongodb::error::Result<()>>,
{
    let mut names = Generator::default();

    let database_name = names.next().unwrap().replace('-', "");
    let connection_string = format!("{}{}?authSource=admin&retryWrites=true", &*CONN_STR, database_name);

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
        preview_features: PreviewFeature::MongoDb.into(),
    };

    RT.block_on(async move {
        let client = Client::with_uri_str(&connection_string).await.unwrap();
        let database = client.database(&database_name);
        let connector = MongoDbIntrospectionConnector::new(&connection_string).await.unwrap();

        if let Err(_) = init_database(database.clone()).await {
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
