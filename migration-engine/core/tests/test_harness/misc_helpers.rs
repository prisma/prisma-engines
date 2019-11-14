use datamodel::ast::{parser, SchemaAst};
use migration_connector::*;
use migration_core::{
    api::{GenericApi, MigrationApi},
    commands::ResetCommand,
};
use sql_connection::{GenericSqlConnection, SyncSqlConnection};
use quaint::pool::SqlFamily;
use sql_migration_connector::{SqlMigrationConnector};
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::{rc::Rc, sync::Arc};
use url::Url;

pub const SCHEMA_NAME: &str = "lift";

pub struct TestSetup {
    pub sql_family: SqlFamily,
    pub database: Arc<dyn SyncSqlConnection + Send + Sync + 'static>,
}

impl TestSetup {
    pub fn is_sqlite(&self) -> bool {
        match self.sql_family {
            SqlFamily::Sqlite => true,
            _ => false,
        }
    }
}

pub fn parse(datamodel_string: &str) -> SchemaAst {
    parser::parse(datamodel_string).unwrap()
}

pub fn test_each_connector<F>(test_fn: F)
where
    F: Fn(&TestSetup, &dyn GenericApi) -> () + std::panic::RefUnwindSafe,
{
    test_each_connector_with_ignores(Vec::new(), test_fn);
}

pub(super) fn mysql_migration_connector(database_url: &str) -> SqlMigrationConnector {
    match SqlMigrationConnector::new_from_database_str(database_url) {
        Ok(c) => c,
        Err(_) => {
            let url = Url::parse(database_url).unwrap();
            let name_cmd = |name| format!("CREATE DATABASE `{}`", name);
            let connect_cmd = |url: url::Url| GenericSqlConnection::from_database_str(url.as_str(), None);

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
            let connect_cmd = |url: url::Url| GenericSqlConnection::from_database_str(url.as_str(), None);

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

pub fn test_each_connector_with_ignores<I: AsRef<[SqlFamily]>, F>(ignores: I, test_fn: F)
where
    F: Fn(&TestSetup, &dyn GenericApi) -> () + std::panic::RefUnwindSafe,
{
    let ignores: &[SqlFamily] = ignores.as_ref();
    // POSTGRES
    if !ignores.contains(&SqlFamily::Postgres) {
        println!("--------------- Testing with Postgres now ---------------");

        let connector = postgres_migration_connector(&postgres_url());

        let test_setup = TestSetup {
            sql_family: SqlFamily::Postgres,
            database: Arc::clone(&connector.database),
        };

        let api = test_api(connector);

        test_fn(&test_setup, &api);
    } else {
        println!("--------------- Ignoring Postgres ---------------")
    }

    // MYSQL
    if !ignores.contains(&SqlFamily::Mysql) {
        println!("--------------- Testing with MySQL now ---------------");

        let connector = mysql_migration_connector(&mysql_url());

        let test_setup = TestSetup {
            sql_family: SqlFamily::Mysql,
            database: Arc::clone(&connector.database),
        };
        let api = test_api(connector);

        test_fn(&test_setup, &api);

        println!("--------------- Testing with MySQL 8 now ---------------");

        let connector = mysql_migration_connector(&mysql_8_url());

        let test_setup = TestSetup {
            sql_family: SqlFamily::Mysql,
            database: Arc::clone(&connector.database),
        };
        let api = test_api(connector);

        test_fn(&test_setup, &api);
    } else {
        println!("--------------- Ignoring MySQL ---------------")
    }

    // SQLite
    if !ignores.contains(&SqlFamily::Sqlite) {
        println!("--------------- Testing with SQLite now ---------------");

        let connector = sqlite_migration_connector();
        let test_setup = TestSetup {
            sql_family: SqlFamily::Sqlite,
            database: Arc::clone(&connector.database),
        };
        let api = test_api(connector);

        test_fn(&test_setup, &api);
    } else {
        println!("--------------- Ignoring SQLite ---------------")
    }
}

pub fn test_api<C, D>(connector: C) -> MigrationApi<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + Send + Sync + 'static,
{
    let api = MigrationApi::new(connector).unwrap();

    api.handle_command::<ResetCommand>(&serde_json::Value::Null)
        .expect("Engine reset failed");

    api
}

pub fn introspect_database(test_setup: &TestSetup, api: &dyn GenericApi) -> SqlSchema {
    let db = Arc::clone(&test_setup.database);
    let inspector: Box<dyn SqlSchemaDescriberBackend> = match api.connector_type() {
        "postgresql" => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(db)),
        "sqlite" => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(db)),
        "mysql" => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(db)),
        _ => unimplemented!(),
    };

    let mut result = inspector
        .describe(&SCHEMA_NAME.to_string())
        .expect("Introspection failed");

    // The presence of the _Migration table makes assertions harder. Therefore remove it from the result.
    result.tables = result.tables.into_iter().filter(|t| t.name != "_Migration").collect();

    result
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

            let connect_cmd = |url: url::Url| GenericSqlConnection::from_database_str(url.as_str(), None);

            let conn = with_database(url, "postgres", "postgres", create_cmd, Rc::new(connect_cmd));

            Arc::new(conn)
        }
        SqlFamily::Sqlite => Arc::new(GenericSqlConnection::from_database_str(database_url, Some(SCHEMA_NAME)).unwrap()),
        SqlFamily::Mysql => {
            let url = Url::parse(database_url).unwrap();
            let create_cmd = |name| format!("CREATE DATABASE `{}`", name);

            let connect_cmd = |url: url::Url| GenericSqlConnection::from_database_str(url.as_str(), None);

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
