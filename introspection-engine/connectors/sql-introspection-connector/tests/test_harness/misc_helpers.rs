use crate::test_harness::{GenericSqlConnection, SqlConnection};
use barrel::Migration;
use pretty_assertions::assert_eq;
use std::{rc::Rc, sync::Arc};
use url::Url;
use once_cell::sync::Lazy;

pub static TEST_ASYNC_RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().expect("failed to start tokio test runtime"));

pub(crate) fn custom_assert(left: &str, right: &str) {
    let parsed_expected = datamodel::parse_datamodel(&right).unwrap();
    let reformatted_expected =
        datamodel::render_datamodel_to_string(&parsed_expected).expect("Datamodel rendering failed");

    assert_eq!(left, reformatted_expected);
}

async fn run_full_sql(database: &Arc<dyn SqlConnection + Send + Sync>, full_sql: &str) {
    for sql in full_sql.split(";") {
        if sql != "" {
            database.query_raw(&sql, &[]).await.unwrap();
        }
    }
}

// barrel

pub struct BarrelMigrationExecutor {
    pub(super) database: Arc<dyn SqlConnection + Send + Sync>,
    pub(super) sql_variant: barrel::backend::SqlVariant,
}

impl BarrelMigrationExecutor {
    pub async fn execute<F>(&self, mut migration_fn: F)
    where
        F: FnMut(&mut Migration) -> (),
    {
        let mut migration = Migration::new().schema(SCHEMA_NAME);
        migration_fn(&mut migration);
        let full_sql = dbg!(migration.make_from(self.sql_variant));
        run_full_sql(&self.database, &full_sql).await;
    }
}

// get dbs

pub async fn database(database_url: &str) -> Box<dyn SqlConnection + Send + Sync + 'static> {
    let url: Url = database_url.parse().unwrap();

    let boxed: Box<dyn SqlConnection + Send + Sync + 'static> = match url.scheme() {
        "postgresql" | "postgres" => {
            let url = Url::parse(database_url).unwrap();
            let create_cmd = |name| format!("CREATE DATABASE \"{}\"", name);

            let connect_cmd = |url: Url| GenericSqlConnection::from_database_str(url.as_str(), None);

            let conn = with_database(url, "postgres", "postgres", create_cmd, Rc::new(connect_cmd)).await;

            Box::new(conn)
        }
        "mysql" => {
            let url = Url::parse(database_url).unwrap();
            let create_cmd = |name| format!("CREATE DATABASE `{}`", name);

            let connect_cmd = |url: Url| GenericSqlConnection::from_database_str(url.as_str(), None);

            let conn = with_database(url, "mysql", "/", create_cmd, Rc::new(connect_cmd)).await;

            Box::new(conn)
        }
        "file" | "sqlite" => {
            Box::new(GenericSqlConnection::from_database_str(database_url, Some("introspection-engine")).unwrap())
        }
        scheme => panic!("Unknown scheme `{}° in database URL: {}", scheme, url.as_str()),
    };

    boxed
}

async fn with_database<F, T, S>(url: Url, default_name: &str, root_path: &str, create_stmt: S, f: Rc<F>) -> T
where
    T: SqlConnection,
    F: Fn(Url) -> Result<T, quaint::error::Error>,
    S: FnOnce(String) -> String,
{
    match f(url.clone()) {
        Ok(conn) => conn,
        Err(_) => {
            create_database(url.clone(), default_name, root_path, create_stmt, f.clone()).await;
            f(url).unwrap()
        }
    }
}

async fn create_database<F, T, S>(url: Url, default_name: &str, root_path: &str, create_stmt: S, f: Rc<F>)
where
    T: SqlConnection,
    F: Fn(Url) -> Result<T, quaint::error::Error>,
    S: FnOnce(String) -> String,
{
    let db_name = fetch_db_name(&url, default_name);

    let mut url = url.clone();
    url.set_path(root_path);

    let conn = f(url).unwrap();

    conn.execute_raw(&create_stmt(db_name), &[]).await.unwrap();
}

fn fetch_db_name(url: &Url, default: &str) -> String {
    let result = match url.path_segments() {
        Some(mut segments) => segments.next().unwrap_or(default),
        None => default,
    };

    String::from(result)
}

// urls
pub const SCHEMA_NAME: &str = "introspection-engine";

pub fn sqlite_test_url() -> String {
    format!("file:{}", sqlite_test_file())
}

pub fn sqlite_test_file() -> String {
    let server_root = std::env::var("SERVER_ROOT").expect("Env var SERVER_ROOT required but not found.");
    let database_folder_path = format!("{}/db", server_root);
    let file_path = format!("{}/{}.db", database_folder_path, SCHEMA_NAME);
    file_path
}

pub fn postgres_url() -> String {
    dbg!(format!(
        "postgresql://postgres:prisma@{}:5432/test-db?schema={}",
        db_host_postgres(),
        SCHEMA_NAME
    ))
}

pub fn mysql_url() -> String {
    dbg!(format!("mysql://root:prisma@{}:3306/", db_host_mysql_5_7()))
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
