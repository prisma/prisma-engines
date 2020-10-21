use crate::{
    context::PrismaContext,
    request_handlers::{graphql, GraphQlBody, SingleQuery},
    PrismaResponse,
};
use migration_core::{
    api::{GenericApi, MigrationApi},
    commands::SchemaPushInput,
};
use quaint::{
    ast::*,
    connector::ConnectionInfo,
    visitor::{self, Visitor},
};
use sql_migration_connector::{sql_migration::SqlMigration, SqlMigrationConnector};
use std::sync::Arc;
use test_setup::*;

pub struct QueryEngine {
    context: Arc<PrismaContext>,
}

impl QueryEngine {
    #[allow(dead_code)]
    pub fn new(ctx: PrismaContext) -> Self {
        QueryEngine { context: Arc::new(ctx) }
    }

    pub async fn request(&self, body: impl Into<SingleQuery>) -> serde_json::Value {
        let body = GraphQlBody::Single(body.into());
        let cx = self.context.clone();
        match graphql::handle(body, cx).await {
            PrismaResponse::Single(response) => serde_json::to_value(response).unwrap(),
            _ => unreachable!(),
        }
    }
}

pub struct TestApi {
    connection_info: ConnectionInfo,
    migration_api: MigrationApi<SqlMigrationConnector, SqlMigration>,
    config: String,
}

impl TestApi {
    pub async fn create_engine(&self, datamodel: &str) -> anyhow::Result<QueryEngine> {
        feature_flags::initialize(&[String::from("all")]).unwrap();

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
        &self.connection_info
    }

    pub fn to_sql_string<'a>(&'a self, query: impl Into<Query<'a>>) -> quaint::Result<(String, Vec<Value>)> {
        match self.connection_info() {
            ConnectionInfo::Postgres(..) => visitor::Postgres::build(query),
            ConnectionInfo::Mysql(..) => visitor::Mysql::build(query),
            ConnectionInfo::Sqlite { .. } => visitor::Sqlite::build(query),
            ConnectionInfo::Mssql(_) => visitor::Mssql::build(query),
        }
    }
}

pub async fn mysql_8_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = mysql_8_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(mysql_migration_connector(&url).await).await.unwrap();

    let config = mysql_8_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn mysql_5_6_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = mysql_5_6_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(mysql_migration_connector(&url).await).await.unwrap();

    let config = mysql_5_6_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn mysql_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = mysql_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(mysql_migration_connector(&url).await).await.unwrap();

    let config = mysql_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn mysql_mariadb_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = mariadb_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(mysql_migration_connector(&url).await).await.unwrap();

    let config = mariadb_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn postgres9_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = postgres_9_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(postgres_migration_connector(&url).await)
        .await
        .unwrap();

    let config = postgres_9_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn postgres_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = postgres_10_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(postgres_migration_connector(&url).await)
        .await
        .unwrap();

    let config = postgres_10_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn postgres11_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = postgres_11_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(postgres_migration_connector(&url).await)
        .await
        .unwrap();

    let config = postgres_11_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn postgres12_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = postgres_12_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(postgres_migration_connector(&url).await)
        .await
        .unwrap();

    let config = postgres_12_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn postgres13_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = postgres_13_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(postgres_migration_connector(&url).await)
        .await
        .unwrap();

    let config = postgres_13_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn sqlite_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = sqlite_test_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(sqlite_migration_connector(db_name).await)
        .await
        .unwrap();

    let config = sqlite_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn mssql_2017_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = mssql_2017_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(mssql_migration_connector(&url).await).await.unwrap();

    let config = mssql_2017_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub async fn mssql_2019_test_api(args: TestAPIArgs) -> TestApi {
    let db_name = args.test_function_name;
    let url = mssql_2019_url(db_name);
    let connection_info = ConnectionInfo::from_url(&url).unwrap();

    let migration_api = MigrationApi::new(mssql_migration_connector(&url).await).await.unwrap();

    let config = mssql_2019_test_config(db_name);

    TestApi {
        connection_info,
        migration_api,
        config,
    }
}

pub(super) async fn mysql_migration_connector(url_str: &str) -> SqlMigrationConnector {
    create_mysql_database(&url_str.parse().unwrap()).await.unwrap();
    SqlMigrationConnector::new(url_str).await.unwrap()
}

pub(super) async fn mssql_migration_connector(url_str: &str) -> SqlMigrationConnector {
    create_mssql_database(url_str).await.unwrap();
    SqlMigrationConnector::new(url_str).await.unwrap()
}

pub(super) async fn postgres_migration_connector(url_str: &str) -> SqlMigrationConnector {
    create_postgres_database(&url_str.parse().unwrap()).await.unwrap();
    SqlMigrationConnector::new(url_str).await.unwrap()
}

pub(super) async fn sqlite_migration_connector(db_name: &str) -> SqlMigrationConnector {
    SqlMigrationConnector::new(&sqlite_test_url(db_name)).await.unwrap()
}
