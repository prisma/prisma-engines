use datamodel::ast::{parser, SchemaAst};
use migration_connector::*;
use migration_core::{
    api::{GenericApi, MigrationApi},
    commands::ResetCommand,
};
use sql_connection::{Mysql, Postgresql, Sqlite, SyncSqlConnection};
use sql_migration_connector::{SqlFamily, SqlMigrationConnector};
use std::{rc::Rc, sync::Arc};
use url::Url;

pub const SCHEMA_NAME: &str = "lift";

pub fn parse(datamodel_string: &str) -> SchemaAst {
    parser::parse(datamodel_string).unwrap()
}

pub(super) fn mysql_migration_connector(database_url: &str) -> SqlMigrationConnector {
    match SqlMigrationConnector::new_from_database_str(database_url) {
        Ok(c) => c,
        Err(_) => {
            let url = Url::parse(database_url).unwrap();

            let name_cmd = |name| format!("CREATE DATABASE `{}`", name);

            let connect_cmd = |url| Mysql::new(url);

            create_database(url, "mysql", "/", name_cmd, Rc::new(connect_cmd));
            SqlMigrationConnector::new_from_database_str(database_url).unwrap()
        }
    }
}

pub(super) fn postgres_migration_connector(url: &str) -> SqlMigrationConnector {
    match SqlMigrationConnector::new_from_database_str(&postgres_url()) {
        Ok(c) => c,
        Err(_) => {
            let name_cmd = |name| format!("CREATE DATABASE \"{}\"", name);

            let connect_cmd = |url: url::Url| Postgresql::new(url);

            create_database(
                url.parse().unwrap(),
                "postgres",
                "postgres",
                name_cmd,
                Rc::new(connect_cmd),
            );
            SqlMigrationConnector::new_from_database_str(&postgres_url()).unwrap()
        }
    }
}

pub(super) fn sqlite_migration_connector() -> SqlMigrationConnector {
    SqlMigrationConnector::new_from_database_str(&sqlite_test_file()).unwrap()
}

pub fn test_api<C, D>(connector: C) -> impl GenericApi
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + Send + Sync + 'static,
{
    let api = MigrationApi::new(connector).unwrap();

    api.handle_command::<ResetCommand>(&serde_json::Value::Null)
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

fn create_database<F, T, S>(url: Url, default_name: &str, root_path: &str, create_stmt: S, f: Rc<F>)
where
    T: SyncSqlConnection,
    F: Fn(Url) -> Result<T, quaint::error::Error>,
    S: FnOnce(String) -> String,
{
    let db_name = fetch_db_name(&url, default_name);

    let mut url = url.clone();
    url.set_path(root_path);

    let conn = f(url).unwrap();

    conn.execute_raw(&create_stmt(db_name), &[]).unwrap();
}

fn with_database<F, T, S>(url: Url, default_name: &str, root_path: &str, create_stmt: S, f: Rc<F>) -> T
where
    T: SyncSqlConnection,
    F: Fn(Url) -> Result<T, quaint::error::Error>,
    S: FnOnce(String) -> String,
{
    match f(url.clone()) {
        Ok(conn) => conn,
        Err(_) => {
            create_database(url.clone(), default_name, root_path, create_stmt, f.clone());
            f(url).unwrap()
        }
    }
}

pub fn database(sql_family: SqlFamily, database_url: &str) -> Arc<dyn SyncSqlConnection + Send + Sync + 'static> {
    match sql_family {
        SqlFamily::Postgres => {
            let url = Url::parse(database_url).unwrap();
            let create_cmd = |name| format!("CREATE DATABASE \"{}\"", name);

            let connect_cmd = |url| Postgresql::new(url);

            let conn = with_database(url, "postgres", "postgres", create_cmd, Rc::new(connect_cmd));

            Arc::new(conn)
        }
        SqlFamily::Sqlite => Arc::new(Sqlite::new(database_url, SCHEMA_NAME).unwrap()),
        SqlFamily::Mysql => {
            let url = Url::parse(database_url).unwrap();
            let create_cmd = |name| format!("CREATE DATABASE `{}`", name);

            let connect_cmd = |url| Mysql::new(url);

            let conn = with_database(url, "mysql", "/", create_cmd, Rc::new(connect_cmd));

            Arc::new(conn)
        }
    }
}

pub fn sqlite_test_config() -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "sqlite"
            url = "file:{}"
            default = true
        }}
    "#,
        sqlite_test_file()
    )
}

pub fn sqlite_test_file() -> String {
    let server_root = std::env::var("SERVER_ROOT").expect("Env var SERVER_ROOT required but not found.");
    let database_folder_path = format!("{}/db", server_root);
    let file_path = format!("file://{}/{}.db", database_folder_path, SCHEMA_NAME);
    file_path
}

pub fn postgres_test_config() -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "postgresql"
            url = "{}"
            default = true
        }}
    "#,
        postgres_url()
    )
}

pub fn mysql_test_config() -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "mysql"
            url = "{}"
            default = true
        }}
    "#,
        mysql_url()
    )
}

pub fn postgres_url() -> String {
    dbg!(format!(
        "postgresql://postgres:prisma@{}:5432/test-db?schema={}",
        db_host_postgres(),
        SCHEMA_NAME
    ))
}

pub fn mysql_url() -> String {
    dbg!(format!(
        "mysql://root:prisma@{host}:3306/{schema_name}",
        host = db_host_mysql_5_7(),
        schema_name = SCHEMA_NAME
    ))
}

pub fn mysql_8_url() -> String {
    let (host, port) = db_host_and_port_mysql_8_0();
    dbg!(format!(
        "mysql://root:prisma@{host}:{port}/{schema_name}",
        host = host,
        port = port,
        schema_name = SCHEMA_NAME
    ))
}

fn db_host_postgres() -> String {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-postgres".to_string(),
        Err(_) => "127.0.0.1".to_string(),
    }
}

fn db_host_and_port_mysql_8_0() -> (String, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mysql-8-0".to_string(), 3306),
        Err(_) => ("127.0.0.1".to_string(), 3307),
    }
}

fn db_host_mysql_5_7() -> String {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-mysql-5-7".to_string(),
        Err(_) => "127.0.0.1".to_string(),
    }
}
