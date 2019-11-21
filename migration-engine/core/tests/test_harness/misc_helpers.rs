use datamodel::ast::{parser, SchemaAst};
use migration_connector::*;
use migration_core::{api::MigrationApi, commands::ResetCommand};
use once_cell::sync::Lazy;
use quaint::{prelude::*};
use sql_migration_connector::SqlMigrationConnector;
use std::{rc::Rc};
use url::Url;
use test_setup::*;

pub static TEST_ASYNC_RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().expect("failed to start tokio test runtime"));

pub fn parse(datamodel_string: &str) -> SchemaAst {
    parser::parse(datamodel_string).unwrap()
}

pub(super) async fn mysql_migration_connector(database_url: &str) -> SqlMigrationConnector {
    match SqlMigrationConnector::new(database_url).await {
        Ok(c) => c,
        Err(_) => {
            let url = Url::parse(database_url).unwrap();
            let name_cmd = |name| format!("CREATE DATABASE `{}`", name);
            let connect_cmd = |url: url::Url| Quaint::new(url.as_str());

            create_database(url, "mysql", "/", name_cmd, Rc::new(connect_cmd)).await;
            SqlMigrationConnector::new(database_url)
                .await
                .unwrap()
        }
    }
}

pub(super) async fn postgres_migration_connector(url: &str) -> SqlMigrationConnector {
    match SqlMigrationConnector::new(&postgres_url()).await {
        Ok(c) => c,
        Err(_) => {
            let name_cmd = |name| format!("CREATE DATABASE \"{}\"", name);
            let connect_cmd = |url: url::Url| Quaint::new(url.as_str());

            create_database(
                url.parse().unwrap(),
                "postgres",
                "postgres",
                name_cmd,
                Rc::new(connect_cmd),
            )
            .await;
            SqlMigrationConnector::new(&postgres_url())
                .await
                .unwrap()
        }
    }
}

pub(super) async fn sqlite_migration_connector() -> SqlMigrationConnector {
    SqlMigrationConnector::new(&sqlite_test_url())
        .await
        .unwrap()
}

pub async fn test_api<C, D>(connector: C) -> MigrationApi<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + Send + Sync + 'static,
{
    let api = MigrationApi::new(connector).await.unwrap();

    api.handle_command::<ResetCommand>(&serde_json::Value::Null)
        .await
        .expect("Engine reset failed");

    api
}

fn fetch_db_name(url: &Url, default: &str) -> String {
    let result = match url.path_segments() {
        Some(mut segments) => segments.next().unwrap_or(default),
        None => default,
    };

    String::from(result)
}

async fn create_database<F, T, S>(url: Url, default_name: &str, root_path: &str, create_stmt: S, f: Rc<F>)
where
    T: Queryable,
    F: Fn(Url) -> Result<T, quaint::error::Error>,
    S: FnOnce(String) -> String,
{
    let db_name = fetch_db_name(&url, default_name);

    let mut url = url.clone();
    url.set_path(root_path);

    let conn = f(url).unwrap();

    conn.execute_raw(&create_stmt(db_name), &[]).await.unwrap();
}
