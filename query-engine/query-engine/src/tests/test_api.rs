pub use test_setup::{BitFlags, Capabilities, Tags};

use crate::context::PrismaContext;
use migration_core::{api::GenericApi, commands::SchemaPushInput};
use quaint::{
    ast::*,
    connector::ConnectionInfo,
    visitor::{self, Visitor},
};
use request_handlers::{GraphQlBody, GraphQlHandler, PrismaResponse, SingleQuery};
use sql_migration_connector::SqlMigrationConnector;
use std::sync::Arc;
use test_setup::{sqlite_test_url, TestApiArgs};

pub struct QueryEngine {
    context: Arc<PrismaContext>,
}

impl QueryEngine {
    pub async fn request(&self, body: impl Into<SingleQuery>) -> serde_json::Value {
        let body = GraphQlBody::Single(body.into());
        let cx = self.context.clone();

        let handler = GraphQlHandler::new(&*cx.executor, cx.query_schema());

        match handler.handle(body).await {
            PrismaResponse::Single(response) => serde_json::to_value(response).unwrap(),
            _ => unreachable!(),
        }
    }
}

pub struct TestApi {
    migration_api: SqlMigrationConnector,
    config: String,
}

impl TestApi {
    pub async fn new(args: TestApiArgs) -> Self {
        let tags = args.tags();

        let (migration_api, url) = if tags.contains(Tags::Mysql) {
            mysql_migration_connector(&args).await
        } else if tags.contains(Tags::Postgres) {
            postgres_migration_connector(&args).await
        } else if tags.contains(Tags::Sqlite) {
            sqlite_migration_connector(args.test_function_name()).await
        } else if tags.contains(Tags::Mssql) {
            mssql_migration_connector(args.test_function_name(), &args).await
        } else {
            unreachable!()
        };

        TestApi {
            migration_api,
            config: args.datasource_block(&url, &[]).to_string(),
        }
    }

    pub async fn create_engine(&self, datamodel: &str) -> anyhow::Result<QueryEngine> {
        let datamodel_string = format!("{}\n\n{}", self.config, datamodel);
        let dml = datamodel::parse_datamodel(&datamodel_string).unwrap().subject;
        let config = datamodel::parse_configuration(&datamodel_string).unwrap();

        self.migration_api
            .schema_push(&SchemaPushInput {
                schema: datamodel_string,
                force: true,
                assume_empty: true,
            })
            .await?;

        let context = PrismaContext::builder(config.subject, dml)
            .enable_raw_queries(true)
            .build()
            .await
            .unwrap();

        Ok(QueryEngine {
            context: Arc::new(context),
        })
    }

    pub fn connection_info(&self) -> &ConnectionInfo {
        self.migration_api.quaint().connection_info()
    }

    pub fn to_sql_string<'a>(&'a self, query: impl Into<Query<'a>>) -> quaint::Result<(String, Vec<Value>)> {
        match self.connection_info() {
            ConnectionInfo::Postgres(..) => visitor::Postgres::build(query),
            ConnectionInfo::Mysql(..) => visitor::Mysql::build(query),
            ConnectionInfo::Sqlite { .. } | ConnectionInfo::InMemorySqlite { .. } => visitor::Sqlite::build(query),
            ConnectionInfo::Mssql(_) => visitor::Mssql::build(query),
        }
    }

    pub fn table_name<'a>(&'a self, name: &'a str) -> quaint::ast::Table<'a> {
        match self.connection_info() {
            ConnectionInfo::Mssql(url) => (url.schema(), name).into(),
            _ => name.into(),
        }
    }
}

pub(super) async fn mysql_migration_connector(args: &TestApiArgs) -> (SqlMigrationConnector, String) {
    let (_db_name, url) = args.create_mysql_database().await;
    (SqlMigrationConnector::new(&url, None).await.unwrap(), url)
}

pub(super) async fn mssql_migration_connector(db_name: &str, args: &TestApiArgs) -> (SqlMigrationConnector, String) {
    let (_, url) = test_setup::init_mssql_database(args.database_url(), db_name)
        .await
        .unwrap();
    (SqlMigrationConnector::new(&url, None).await.unwrap(), url)
}

pub(super) async fn postgres_migration_connector(args: &TestApiArgs) -> (SqlMigrationConnector, String) {
    let (_db_name, _, url) = args.create_postgres_database().await;
    (SqlMigrationConnector::new(&url, None).await.unwrap(), url)
}

pub(super) async fn sqlite_migration_connector(db_name: &str) -> (SqlMigrationConnector, String) {
    let url = sqlite_test_url(db_name);
    (SqlMigrationConnector::new(&url, None).await.unwrap(), url)
}
