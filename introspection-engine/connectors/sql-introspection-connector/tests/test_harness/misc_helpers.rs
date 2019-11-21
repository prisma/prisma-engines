use barrel::Migration;
use once_cell::sync::Lazy;
use quaint::prelude::*;
use std::{rc::Rc, sync::Arc};
use test_setup::*;
use url::Url;

pub static TEST_ASYNC_RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().expect("failed to start tokio test runtime"));

pub(crate) fn custom_assert(left: &str, right: &str) {
    let parsed_expected = datamodel::parse_datamodel(&right).unwrap();
    let reformatted_expected =
        datamodel::render_datamodel_to_string(&parsed_expected).expect("Datamodel rendering failed");

    assert_eq!(left, reformatted_expected);
}

async fn run_full_sql(database: &Arc<dyn Queryable + Send + Sync>, full_sql: &str) {
    for sql in full_sql.split(";") {
        if sql != "" {
            database.query_raw(&sql, &[]).await.unwrap();
        }
    }
}

// barrel

pub struct BarrelMigrationExecutor {
    pub(super) database: Arc<dyn Queryable + Send + Sync>,
    pub(super) sql_variant: barrel::backend::SqlVariant,
}

impl BarrelMigrationExecutor {
    pub async fn execute<F>(&self, migration_fn: F)
    where
        F: FnMut(&mut Migration) -> (),
    {
        self.execute_with_schema(migration_fn, SCHEMA_NAME).await
    }

    pub async fn execute_with_schema<F>(&self, mut migration_fn: F, schema_name: &str)
    where
        F: FnMut(&mut Migration) -> (),
    {
        let mut migration = Migration::new().schema(schema_name);
        migration_fn(&mut migration);
        let full_sql = migration.make_from(self.sql_variant);
        run_full_sql(&self.database, &full_sql).await;
    }
}

// get dbs

pub async fn database(database_url: &str) -> Box<dyn Queryable + Send + Sync + 'static> {
    let url: Url = database_url.parse().unwrap();

    let boxed: Box<dyn Queryable + Send + Sync + 'static> = match url.scheme() {
        "postgresql" | "postgres" => {
            let create_cmd = |name| format!("CREATE DATABASE \"{}\"", name);

            let connect_cmd = |url: Url| Quaint::new(url.as_str());

            let conn = with_database(url, "postgres", "postgres", create_cmd, Rc::new(connect_cmd)).await;

            Box::new(conn)
        }
        "mysql" => {
            let create_cmd = |name| format!("CREATE DATABASE `{}`", name);

            let connect_cmd = |url: Url| Quaint::new(url.as_str());

            let conn = with_database(url, "mysql", "/", create_cmd, Rc::new(connect_cmd)).await;

            Box::new(conn)
        }
        "file" | "sqlite" => Box::new(Quaint::new(url.as_str()).unwrap()),
        scheme => panic!("Unknown scheme `{}° in database URL: {}", scheme, url.as_str()),
    };

    boxed
}

async fn with_database<F, T, S>(url: Url, default_name: &str, root_path: &str, create_stmt: S, f: Rc<F>) -> T
where
    T: Queryable,
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

fn fetch_db_name(url: &Url, default: &str) -> String {
    let result = match url.path_segments() {
        Some(mut segments) => segments.next().unwrap_or(default),
        None => default,
    };

    String::from(result)
}
