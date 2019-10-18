mod test_harness;

use barrel::{types, Migration, SqlVariant};
use introspection_connector::IntrospectionConnector;
use pretty_assertions::assert_eq;
use prisma_query::connector::{MysqlParams, PostgresParams};
use sql_introspection_connector::*;
use std::{convert::TryFrom, rc::Rc, sync::Arc};
use test_harness::*;
use url::Url;

pub const SCHEMA_NAME: &str = "introspection-engine";

#[test]
fn adding_a_model_for_an_existing_table_must_work() {
    test_each_backend(|test_setup, barrel| {
        let initial_result = barrel.execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        });
        let dm = r#"
            model Blog {
                id Int @id
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(result, dm.to_string());
    });
}

fn custom_assert(left: String, right: String) {
    let parsed_expected = datamodel::parse_datamodel(&right).unwrap();
    let reformatted_expected =
        datamodel::render_datamodel_to_string(&parsed_expected).expect("Datamodel rendering failed");

    assert_eq!(left, reformatted_expected);
}

fn introspect(test_setup: &TestSetup) -> String {
    let datamodel = test_setup.introspection_connector.introspect(SCHEMA_NAME).unwrap();
    datamodel::render_datamodel_to_string(&datamodel).expect("Datamodel rendering failed")
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SqlFamily {
    Sqlite,
    Postgres,
    Mysql,
}

pub struct TestSetup {
    pub sql_family: SqlFamily,
    pub database: Arc<dyn IntrospectionDatabase + Send + Sync + 'static>,
    pub introspection_connector: Box<IntrospectionConnector>,
}

struct BarrelMigrationExecutor {
    database: Arc<dyn IntrospectionDatabase + Send + Sync>,
    sql_variant: barrel::backend::SqlVariant,
}

impl BarrelMigrationExecutor {
    fn execute<F>(&self, mut migrationFn: F)
    where
        F: FnMut(&mut Migration) -> (),
    {
        let mut migration = Migration::new().schema(SCHEMA_NAME);
        migrationFn(&mut migration);
        let full_sql = dbg!(migration.make_from(self.sql_variant));
        run_full_sql(&self.database, &full_sql);
    }
}

fn run_full_sql(database: &Arc<dyn IntrospectionDatabase + Send + Sync>, full_sql: &str) {
    for sql in full_sql.split(";") {
        if sql != "" {
            database.query_raw(SCHEMA_NAME, &sql, &[]).unwrap();
        }
    }
}

fn test_each_backend<F>(test_fn: F)
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
        let connector = SqlIntrospectionConnector::new(&sqlite_test_url()).unwrap();

        let barrel_migration_executor = BarrelMigrationExecutor {
            database: Arc::clone(&test_setup.database),
            sql_variant: SqlVariant::Sqlite,
        };

        test_fn(&test_setup, &barrel_migration_executor);
    } else {
        println!("Ignoring SQLite")
    }
    //    // POSTGRES
    //    if !ignores.contains(&SqlFamily::Postgres) {
    //        println!("Testing with Postgres now");
    //        let (inspector, test_setup) = get_postgres();
    //
    //        println!("Running the test function now");
    //        let connector = SqlMigrationConnector::postgres(&postgres_url(), false).unwrap();
    //        let barrel_migration_executor = BarrelMigrationExecutor {
    //            database: Arc::clone(&test_setup.database),
    //            sql_variant: SqlVariant::Pg,
    //        };
    //
    //        test_fn(&test_setup, &barrel_migration_executor);
    //    } else {
    //        println!("Ignoring Postgres")
    //    }
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

fn sqlite_test_url() -> String {
    format!("file:{}", sqlite_test_file())
}

pub fn sqlite_test_file() -> String {
    let server_root = std::env::var("SERVER_ROOT").expect("Env var SERVER_ROOT required but not found.");
    let database_folder_path = format!("{}/db", server_root);
    let file_path = format!("{}/{}.db", database_folder_path, SCHEMA_NAME);
    file_path
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
