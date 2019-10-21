//use datamodel;
//use migration_connector::*;
//use migration_core::{
//    api::{GenericApi, MigrationApi},
//    commands::ResetCommand,
//    parse_datamodel,
//};
//use prisma_query::connector::{MysqlParams, PostgresParams};
//use sql_migration_connector::{migration_database::*, SqlFamily, SqlMigrationConnector};
//use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
//use std::{convert::TryFrom, rc::Rc, sync::Arc};
//use url::Url;

use crate::test_harness::{IntrospectionDatabase, IntrospectionDatabaseWrapper, Mysql, PostgreSql, Sqlite};
use barrel::{Migration, SqlVariant};
use introspection_connector::IntrospectionConnector;
use pretty_assertions::assert_eq;
use prisma_query::connector::{MysqlParams, PostgresParams};
use sql_introspection_connector::*;
use std::{convert::TryFrom, rc::Rc, sync::Arc};
use url::Url;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SqlFamily {
    Sqlite,
    Postgres,
    Mysql,
}

pub struct TestSetup {
    pub sql_family: SqlFamily,
    pub database: Arc<dyn IntrospectionDatabase + Send + Sync + 'static>,
    pub introspection_connector: Box<dyn IntrospectionConnector>,
}

pub struct BarrelMigrationExecutor {
    database: Arc<dyn IntrospectionDatabase + Send + Sync>,
    sql_variant: barrel::backend::SqlVariant,
}

// test execution

pub(crate) fn custom_assert(left: String, right: String) {
    let parsed_expected = datamodel::parse_datamodel(&right).unwrap();
    let reformatted_expected =
        datamodel::render_datamodel_to_string(&parsed_expected).expect("Datamodel rendering failed");

    assert_eq!(left, reformatted_expected);
}

pub(crate) fn introspect(test_setup: &TestSetup) -> String {
    let datamodel = test_setup.introspection_connector.introspect(SCHEMA_NAME).unwrap();
    datamodel::render_datamodel_to_string(&datamodel).expect("Datamodel rendering failed")
}

fn run_full_sql(database: &Arc<dyn IntrospectionDatabase + Send + Sync>, full_sql: &str) {
    for sql in full_sql.split(";") {
        if sql != "" {
            database.query_raw(SCHEMA_NAME, &sql, &[]).unwrap();
        }
    }
}

pub(crate) fn test_each_backend<F>(test_fn: F)
where
    F: Fn(&TestSetup, &BarrelMigrationExecutor) -> () + std::panic::RefUnwindSafe,
{
    test_each_backend_with_ignores(Vec::new(), test_fn);
}

fn test_each_backend_with_ignores<F>(ignores: Vec<SqlFamily>, test_fn: F)
where
    F: Fn(&TestSetup, &BarrelMigrationExecutor) -> () + std::panic::RefUnwindSafe,
{
    //     SQLite
    if !ignores.contains(&SqlFamily::Sqlite) {
        println!("Testing with SQLite now");
        let test_setup = get_sqlite();

        println!("Running the test function now");
        let barrel_migration_executor = BarrelMigrationExecutor {
            database: Arc::clone(&test_setup.database),
            sql_variant: SqlVariant::Sqlite,
        };

        test_fn(&test_setup, &barrel_migration_executor);
    } else {
        println!("Ignoring SQLite")
    }
    // POSTGRES
    if !ignores.contains(&SqlFamily::Postgres) {
        println!("Testing with Postgres now");
        let test_setup = get_postgres();

        println!("Running the test function now");
        let barrel_migration_executor = BarrelMigrationExecutor {
            database: Arc::clone(&test_setup.database),
            sql_variant: SqlVariant::Pg,
        };

        test_fn(&test_setup, &barrel_migration_executor);
    } else {
        println!("Ignoring Postgres")
    }
    // MySQL
    if !ignores.contains(&SqlFamily::Mysql) {
        println!("Testing with MySql now");
        let test_setup = get_mysql();
        println!("Running the test function now");
        let barrel_migration_executor = BarrelMigrationExecutor {
            database: Arc::clone(&test_setup.database),
            sql_variant: SqlVariant::Mysql,
        };

        test_fn(&test_setup, &barrel_migration_executor);
    } else {
        println!("Ignoring MySql")
    }
}

// barrel

impl BarrelMigrationExecutor {
    pub fn execute<F>(&self, mut migration_Fn: F)
    where
        F: FnMut(&mut Migration) -> (),
    {
        let mut migration = Migration::new().schema(SCHEMA_NAME);
        migration_Fn(&mut migration);
        let full_sql = dbg!(migration.make_from(self.sql_variant));
        run_full_sql(&self.database, &full_sql);
    }
}

// get dbs

pub fn database(sql_family: SqlFamily, database_url: &str) -> Box<dyn IntrospectionDatabase + Send + Sync + 'static> {
    match sql_family {
        SqlFamily::Postgres => {
            let url = Url::parse(database_url).unwrap();
            let create_cmd = |name| format!("CREATE DATABASE \"{}\"", name);

            let connect_cmd = |url| {
                let params = PostgresParams::try_from(url)?;
                PostgreSql::new(params, true)
            };

            let conn = with_database(url, "postgres", "postgres", create_cmd, Rc::new(connect_cmd));

            Box::new(conn)
        }
        SqlFamily::Sqlite => Box::new(Sqlite::new(database_url).unwrap()),
        SqlFamily::Mysql => {
            let url = Url::parse(database_url).unwrap();
            let create_cmd = |name| format!("CREATE DATABASE `{}`", name);

            let connect_cmd = |url| {
                let params = MysqlParams::try_from(url)?;
                Mysql::new(params, true)
            };

            let conn = with_database(url, "mysql", "/", create_cmd, Rc::new(connect_cmd));

            Box::new(conn)
        }
    }
}

pub fn database_wrapper(sql_family: SqlFamily, database_url: &str) -> IntrospectionDatabaseWrapper {
    IntrospectionDatabaseWrapper {
        database: database(sql_family, database_url).into(),
    }
}

fn with_database<F, T, S>(url: Url, default_name: &str, root_path: &str, create_stmt: S, f: Rc<F>) -> T
where
    T: IntrospectionDatabase,
    F: Fn(Url) -> Result<T, prisma_query::error::Error>,
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

fn create_database<F, T, S>(url: Url, default_name: &str, root_path: &str, create_stmt: S, f: Rc<F>)
where
    T: IntrospectionDatabase,
    F: Fn(Url) -> Result<T, prisma_query::error::Error>,
    S: FnOnce(String) -> String,
{
    let db_name = fetch_db_name(&url, default_name);

    let mut url = url.clone();
    url.set_path(root_path);

    let conn = f(url).unwrap();

    conn.execute_raw("", &create_stmt(db_name), &[]).unwrap();
}

fn fetch_db_name(url: &Url, default: &str) -> String {
    let result = match url.path_segments() {
        Some(mut segments) => segments.next().unwrap_or(default),
        None => default,
    };

    String::from(result)
}

fn get_sqlite() -> TestSetup {
    let wrapper = database_wrapper(SqlFamily::Sqlite, &sqlite_test_file());
    let database = Arc::clone(&wrapper.database);

    let database_file_path = sqlite_test_file();
    let _ = std::fs::remove_file(database_file_path.clone()); // ignore potential errors
    let introspection_connector = SqlIntrospectionConnector::new(&sqlite_test_url()).unwrap();

    TestSetup {
        database,
        sql_family: SqlFamily::Sqlite,
        introspection_connector: Box::new(introspection_connector),
    }
}

fn get_postgres() -> TestSetup {
    let wrapper = database_wrapper(SqlFamily::Postgres, &postgres_url());
    let database = Arc::clone(&wrapper.database);

    let drop_schema = dbg!(format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", SCHEMA_NAME));
    let _ = database.query_raw(SCHEMA_NAME, &drop_schema, &[]);

    let introspection_connector = SqlIntrospectionConnector::new(&postgres_url()).unwrap();

    TestSetup {
        database,
        sql_family: SqlFamily::Postgres,
        introspection_connector: Box::new(introspection_connector),
    }
}

fn get_mysql() -> TestSetup {
    let wrapper = database_wrapper(SqlFamily::Mysql, &mysql_url());
    let database = Arc::clone(&wrapper.database);

    let drop_schema = dbg!(format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", SCHEMA_NAME));
    let _ = database.query_raw(SCHEMA_NAME, &drop_schema, &[]);

    let introspection_connector = SqlIntrospectionConnector::new(&mysql_url()).unwrap();

    TestSetup {
        database,
        sql_family: SqlFamily::Mysql,
        introspection_connector: Box::new(introspection_connector),
    }
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
    dbg!(format!(
        "mysql://root:prisma@{}:3306/{}",
        db_host_mysql_5_7(),
        SCHEMA_NAME
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

//pub const SCHEMA_NAME: &str = "migration-engine";
//
//pub struct TestSetup {
//    pub sql_family: SqlFamily,
//    pub database: Arc<dyn MigrationDatabase + Send + Sync + 'static>,
//}
//
//impl TestSetup {
//    pub fn database_wrapper(&self) -> MigrationDatabaseWrapper {
//        MigrationDatabaseWrapper {
//            database: Arc::clone(&self.database),
//        }
//    }
//}
//
//pub fn parse(datamodel_string: &str) -> datamodel::Datamodel {
//    parse_datamodel(datamodel_string).unwrap()
//}
//
//pub fn test_each_connector<F>(test_fn: F)
//where
//    F: Fn(&TestSetup, &dyn GenericApi) -> () + std::panic::RefUnwindSafe,
//{
//    test_each_connector_with_ignores(Vec::new(), test_fn);
//}
//
//pub fn test_only_connector<F>(sql_family: SqlFamily, test_fn: F)
//where
//    F: Fn(&TestSetup, &dyn GenericApi) -> () + std::panic::RefUnwindSafe,
//{
//    let all = &[SqlFamily::Postgres, SqlFamily::Mysql, SqlFamily::Sqlite];
//    let ignores: Vec<SqlFamily> = all.iter().filter(|f| f != &&sql_family).map(|f| *f).collect();
//
//    test_each_connector_with_ignores(ignores, test_fn);
//}
//
//fn mysql_migration_connector(database_url: &str) -> SqlMigrationConnector {
//    match SqlMigrationConnector::mysql(database_url, true) {
//        Ok(c) => c,
//        Err(_) => {
//            let url = Url::parse(database_url).unwrap();
//
//            let name_cmd = |name| format!("CREATE DATABASE `{}`", name);
//
//            let connect_cmd = |url| {
//                let params = MysqlParams::try_from(url)?;
//                Mysql::new(params, true)
//            };
//
//            create_database(url, "mysql", "/", name_cmd, Rc::new(connect_cmd));
//            SqlMigrationConnector::mysql(database_url, true).unwrap()
//        }
//    }
//}
//
//pub fn test_each_connector_with_ignores<I: AsRef<[SqlFamily]>, F>(ignores: I, test_fn: F)
//where
//    F: Fn(&TestSetup, &dyn GenericApi) -> () + std::panic::RefUnwindSafe,
//{
//    let ignores: &[SqlFamily] = ignores.as_ref();
//    // POSTGRES
//    if !ignores.contains(&SqlFamily::Postgres) {
//        println!("--------------- Testing with Postgres now ---------------");
//
//        let connector = match SqlMigrationConnector::postgres(&postgres_url(), true) {
//            Ok(c) => c,
//            Err(_) => {
//                let url = Url::parse(&postgres_url()).unwrap();
//                let name_cmd = |name| format!("CREATE DATABASE \"{}\"", name);
//
//                let connect_cmd = |url| {
//                    let params = PostgresParams::try_from(url)?;
//                    PostgreSql::new(params, true)
//                };
//
//                create_database(url, "postgres", "postgres", name_cmd, Rc::new(connect_cmd));
//                SqlMigrationConnector::postgres(&postgres_url(), true).unwrap()
//            }
//        };
//
//        let test_setup = TestSetup {
//            sql_family: SqlFamily::Postgres,
//            database: Arc::clone(&connector.database),
//        };
//        let api = test_api(connector);
//
//        test_fn(&test_setup, &api);
//    } else {
//        println!("--------------- Ignoring Postgres ---------------")
//    }
//
//    // MYSQL
//    if !ignores.contains(&SqlFamily::Mysql) {
//        println!("--------------- Testing with MySQL now ---------------");
//
//        let connector = mysql_migration_connector(&mysql_url());
//
//        let test_setup = TestSetup {
//            sql_family: SqlFamily::Mysql,
//            database: Arc::clone(&connector.database),
//        };
//        let api = test_api(connector);
//
//        test_fn(&test_setup, &api);
//
//        println!("--------------- Testing with MySQL 8 now ---------------");
//
//        let connector = mysql_migration_connector(&mysql_8_url());
//
//        let test_setup = TestSetup {
//            sql_family: SqlFamily::Mysql,
//            database: Arc::clone(&connector.database),
//        };
//        let api = test_api(connector);
//
//        test_fn(&test_setup, &api);
//    } else {
//        println!("--------------- Ignoring MySQL ---------------")
//    }
//
//    // SQLite
//    if !ignores.contains(&SqlFamily::Sqlite) {
//        println!("--------------- Testing with SQLite now ---------------");
//
//        let connector = SqlMigrationConnector::sqlite(&sqlite_test_file()).unwrap();
//        let test_setup = TestSetup {
//            sql_family: SqlFamily::Sqlite,
//            database: Arc::clone(&connector.database),
//        };
//        let api = test_api(connector);
//
//        test_fn(&test_setup, &api);
//    } else {
//        println!("--------------- Ignoring SQLite ---------------")
//    }
//}
//
//pub fn test_api<C, D>(connector: C) -> impl GenericApi
//where
//    C: MigrationConnector<DatabaseMigration = D>,
//    D: DatabaseMigrationMarker + Send + Sync + 'static,
//{
//    let api = MigrationApi::new(connector).unwrap();
//
//    api.handle_command::<ResetCommand>(&serde_json::Value::Null)
//        .expect("Engine reset failed");
//
//    api
//}
//
//pub fn introspect_database(test_setup: &TestSetup, api: &dyn GenericApi) -> SqlSchema {
//    let db = Arc::new(test_setup.database_wrapper());
//    let inspector: Box<dyn SqlSchemaDescriberBackend> = match api.connector_type() {
//        "postgresql" => Box::new(sql_schema_describer::postgres::SqlSchemaDescriber::new(db)),
//        "sqlite" => Box::new(sql_schema_describer::sqlite::SqlSchemaDescriber::new(db)),
//        "mysql" => Box::new(sql_schema_describer::mysql::SqlSchemaDescriber::new(db)),
//        _ => unimplemented!(),
//    };
//
//    let mut result = inspector
//        .describe(&SCHEMA_NAME.to_string())
//        .expect("Introspection failed");
//
//    // the presence of the _Migration table makes assertions harder. Therefore remove it from the result.
//    result.tables = result.tables.into_iter().filter(|t| t.name != "_Migration").collect();
//
//    result
//}
//
//pub fn database_wrapper(sql_family: SqlFamily, database_url: &str) -> MigrationDatabaseWrapper {
//    MigrationDatabaseWrapper {
//        database: database(sql_family, database_url).into(),
//    }
//}
//
//fn fetch_db_name(url: &Url, default: &str) -> String {
//    let result = match url.path_segments() {
//        Some(mut segments) => segments.next().unwrap_or(default),
//        None => default,
//    };
//
//    String::from(result)
//}
//
//fn create_database<F, T, S>(url: Url, default_name: &str, root_path: &str, create_stmt: S, f: Rc<F>)
//where
//    T: MigrationDatabase,
//    F: Fn(Url) -> Result<T, prisma_query::error::Error>,
//    S: FnOnce(String) -> String,
//{
//    let db_name = fetch_db_name(&url, default_name);
//
//    let mut url = url.clone();
//    url.set_path(root_path);
//
//    let conn = f(url).unwrap();
//
//    conn.execute_raw("", &create_stmt(db_name), &[]).unwrap();
//}
//
//fn with_database<F, T, S>(url: Url, default_name: &str, root_path: &str, create_stmt: S, f: Rc<F>) -> T
//where
//    T: MigrationDatabase,
//    F: Fn(Url) -> Result<T, prisma_query::error::Error>,
//    S: FnOnce(String) -> String,
//{
//    match f(url.clone()) {
//        Ok(conn) => conn,
//        Err(_) => {
//            create_database(url.clone(), default_name, root_path, create_stmt, f.clone());
//            f(url).unwrap()
//        }
//    }
//}
//
//pub fn database(sql_family: SqlFamily, database_url: &str) -> Box<dyn MigrationDatabase + Send + Sync + 'static> {
//    match sql_family {
//        SqlFamily::Postgres => {
//            let url = Url::parse(database_url).unwrap();
//            let create_cmd = |name| format!("CREATE DATABASE \"{}\"", name);
//
//            let connect_cmd = |url| {
//                let params = PostgresParams::try_from(url)?;
//                PostgreSql::new(params, true)
//            };
//
//            let conn = with_database(url, "postgres", "postgres", create_cmd, Rc::new(connect_cmd));
//
//            Box::new(conn)
//        }
//        SqlFamily::Sqlite => Box::new(Sqlite::new(database_url).unwrap()),
//        SqlFamily::Mysql => {
//            let url = Url::parse(database_url).unwrap();
//            let create_cmd = |name| format!("CREATE DATABASE `{}`", name);
//
//            let connect_cmd = |url| {
//                let params = MysqlParams::try_from(url)?;
//                Mysql::new(params, true)
//            };
//
//            let conn = with_database(url, "mysql", "/", create_cmd, Rc::new(connect_cmd));
//
//            Box::new(conn)
//        }
//    }
//}
//
//pub fn sqlite_test_config() -> String {
//    format!(
//        r#"
//        datasource my_db {{
//            provider = "sqlite"
//            url = "file:{}"
//            default = true
//        }}
//    "#,
//        sqlite_test_file()
//    )
//}
//
//pub fn sqlite_test_file() -> String {
//    let server_root = std::env::var("SERVER_ROOT").expect("Env var SERVER_ROOT required but not found.");
//    let database_folder_path = format!("{}/db", server_root);
//    let file_path = format!("{}/{}.db", database_folder_path, SCHEMA_NAME);
//    file_path
//}
//
//pub fn postgres_test_config() -> String {
//    format!(
//        r#"
//        datasource my_db {{
//            provider = "postgresql"
//            url = "{}"
//            default = true
//        }}
//    "#,
//        postgres_url()
//    )
//}
//
//pub fn mysql_test_config() -> String {
//    format!(
//        r#"
//        datasource my_db {{
//            provider = "mysql"
//            url = "{}"
//            default = true
//        }}
//    "#,
//        mysql_url()
//    )
//}
//
//pub fn postgres_url() -> String {
//    dbg!(format!(
//        "postgresql://postgres:prisma@{}:5432/test-db?schema={}",
//        db_host_postgres(),
//        SCHEMA_NAME
//    ))
//}
//
//pub fn mysql_url() -> String {
//    dbg!(format!(
//        "mysql://root:prisma@{}:3306/{}",
//        db_host_mysql_5_7(),
//        SCHEMA_NAME
//    ))
//}
//
//pub fn mysql_8_url() -> String {
//    let (host, port) = db_host_and_port_mysql_8_0();
//    dbg!(format!(
//        "mysql://root:prisma@{host}:{port}/{schema_name}",
//        host = host,
//        port = port,
//        schema_name = SCHEMA_NAME
//    ))
//}
//
//fn db_host_postgres() -> String {
//    match std::env::var("IS_BUILDKITE") {
//        Ok(_) => "test-db-postgres".to_string(),
//        Err(_) => "127.0.0.1".to_string(),
//    }
//}
//
//fn db_host_and_port_mysql_8_0() -> (String, usize) {
//    match std::env::var("IS_BUILDKITE") {
//        Ok(_) => ("test-db-mysql-8-0".to_string(), 3306),
//        Err(_) => ("127.0.0.1".to_string(), 3307),
//    }
//}
//
//fn db_host_mysql_5_7() -> String {
//    match std::env::var("IS_BUILDKITE") {
//        Ok(_) => "test-db-mysql-5-7".to_string(),
//        Err(_) => "127.0.0.1".to_string(),
//    }
//}
