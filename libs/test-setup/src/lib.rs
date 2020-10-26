#![deny(unsafe_code, rust_2018_idioms)]

//! This crate contains constants and utilities that are useful for writing tests across the
//! engines.

/// Macro utils
#[doc(hidden)]
pub mod logging;

/// Macro utils
#[doc(hidden)]
pub mod runtime;

/// The built-in connectors database.
pub mod connectors;

use crate::connectors::Tags;
use enumflags2::BitFlags;
use once_cell::sync::Lazy;
use quaint::{prelude::Queryable, single::Quaint};
use std::str::FromStr;
use url::Url;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

const SCHEMA_NAME: &str = "prisma-tests";

pub struct TestAPIArgs {
    pub test_function_name: &'static str,
    pub test_tag: BitFlags<Tags>,
}

impl TestAPIArgs {
    pub fn new(name: &'static str, tags: u8) -> Self {
        let tags: BitFlags<Tags> = BitFlags::from_bits(tags).unwrap();
        TestAPIArgs {
            test_function_name: name,
            test_tag: tags,
        }
    }
}

pub fn server_root() -> &'static str {
    static SERVER_ROOT: Lazy<String> =
        Lazy::new(|| std::env::var("SERVER_ROOT").expect("SERVER_ROOT env var is not defined"));

    SERVER_ROOT.as_ref()
}

pub fn sqlite_test_url(db_name: &str) -> String {
    format!("file:{}?db_name={}", sqlite_test_file(db_name), SCHEMA_NAME)
}

pub fn sqlite_test_file(db_name: &str) -> String {
    let database_folder_path = format!("{}/db", server_root());
    let file_path = format!("{}/{}.db", database_folder_path, db_name);

    // Truncate the file.
    std::fs::File::create(&file_path).expect("Failed to create or truncate SQLite database.");

    file_path
}

pub fn postgres_9_url(db_name: &str) -> String {
    let (host, port) = db_host_and_port_postgres_9();

    format!(
        "postgresql://postgres:prisma@{}:{}/{}?schema={}&statement_cache_size=0",
        host, port, db_name, SCHEMA_NAME
    )
}

pub fn pgbouncer_url(db_name: &str) -> String {
    let (host, port) = db_host_and_port_for_pgbouncer();

    format!(
        "postgresql://postgres:prisma@{}:{}/{}?schema={}&pgbouncer=true",
        host, port, db_name, SCHEMA_NAME
    )
}

pub fn postgres_10_url(db_name: &str) -> String {
    let (host, port) = db_host_and_port_postgres_10();

    format!(
        "postgresql://postgres:prisma@{}:{}/{}?schema={}&statement_cache_size=0",
        host, port, db_name, SCHEMA_NAME
    )
}

pub fn postgres_11_url(db_name: &str) -> String {
    let (host, port) = db_host_and_port_postgres_11();

    format!(
        "postgresql://postgres:prisma@{}:{}/{}?schema={}&statement_cache_size=0",
        host, port, db_name, SCHEMA_NAME
    )
}

pub fn postgres_12_url(db_name: &str) -> String {
    let (host, port) = db_host_and_port_postgres_12();

    format!(
        "postgresql://postgres:prisma@{}:{}/{}?schema={}&statement_cache_size=0",
        host, port, db_name, SCHEMA_NAME
    )
}

pub fn postgres_13_url(db_name: &str) -> String {
    let (host, port) = db_host_and_port_postgres_13();

    format!(
        "postgresql://postgres:prisma@{}:{}/{}?schema={}&statement_cache_size=0",
        host, port, db_name, SCHEMA_NAME
    )
}

pub fn mysql_url(db_name: &str) -> String {
    let db_name = mysql_safe_identifier(db_name);

    format!(
        "mysql://root:prisma@{host}:3306/{db_name}?connect_timeout=20&socket_timeout=20",
        host = db_host_mysql_5_7(),
        db_name = db_name,
    )
}

pub fn mysql_8_url(db_name: &str) -> String {
    let (host, port) = db_host_and_port_mysql_8_0();

    // maximum length of identifiers on mysql
    let db_name = mysql_safe_identifier(db_name);

    format!(
        "mysql://root:prisma@{host}:{port}{maybe_slash}{db_name}?connect_timeout=20&socket_timeout=20",
        maybe_slash = if db_name.is_empty() { "" } else { "/" },
        host = host,
        port = port,
        db_name = db_name,
    )
}

pub fn mysql_5_6_url(db_name: &str) -> String {
    let (host, port) = db_host_and_port_mysql_5_6();

    // maximum length of identifiers on mysql
    let db_name = mysql_safe_identifier(db_name);

    format!(
        "mysql://root:prisma@{host}:{port}/{db_name}?connect_timeout=20&socket_timeout=20",
        host = host,
        port = port,
        db_name = db_name,
    )
}

pub fn mariadb_url(db_name: &str) -> String {
    let (host, port) = db_host_and_port_mariadb();

    // maximum length of identifiers on mysql
    let db_name = mysql_safe_identifier(db_name);

    format!(
        "mysql://root:prisma@{host}:{port}/{db_name}?connect_timeout=20&socket_timeout=20",
        host = host,
        port = port,
        db_name = db_name,
    )
}

pub fn mssql_2017_url(db_name: &str) -> String {
    let (host, port) = db_host_mssql_2017();

    format!(
        "sqlserver://{host}:{port};database={db_name};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;sockettimeout=15;isolationLevel=READ UNCOMMITTED;encrypt=DANGER_PLAINTEXT",
        db_name = db_name,
        host = host,
        port = port,
    )
}

pub fn mssql_2019_url(db_name: &str) -> String {
    let (host, port) = db_host_mssql_2019();

    format!(
        "sqlserver://{host}:{port};database={db_name};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;sockettimeout=15;isolationLevel=READ UNCOMMITTED;encrypt=DANGER_PLAINTEXT",
        db_name = db_name,
        host = host,
        port = port,
    )
}

fn db_host_and_port_postgres_9() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-postgres-9", 5432),
        Err(_) => ("127.0.0.1", 5431),
    }
}

fn db_host_and_port_postgres_10() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-postgres-10", 5432),
        Err(_) => ("127.0.0.1", 5432),
    }
}

fn db_host_and_port_for_pgbouncer() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-pgbouncer", 6432),
        Err(_) => ("127.0.0.1", 6432),
    }
}

fn db_host_and_port_postgres_11() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-postgres-11", 5432),
        Err(_) => ("127.0.0.1", 5433),
    }
}

fn db_host_and_port_postgres_12() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-postgres-12", 5432),
        Err(_) => ("127.0.0.1", 5434),
    }
}

fn db_host_and_port_postgres_13() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-postgres-13", 5432),
        Err(_) => ("127.0.0.1", 5435),
    }
}

fn db_host_and_port_mysql_8_0() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mysql-8-0", 3306),
        Err(_) => ("127.0.0.1", 3307),
    }
}

fn db_host_and_port_mysql_5_6() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mysql-5-6", 3306),
        Err(_) => ("127.0.0.1", 3309),
    }
}

fn db_host_mysql_5_7() -> &'static str {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => "test-db-mysql-5-7",
        Err(_) => "127.0.0.1",
    }
}

fn db_host_and_port_mariadb() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mariadb", 3306),
        Err(_) => ("127.0.0.1", 3308),
    }
}

fn db_host_mssql_2017() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mssql-2017", 1433),
        Err(_) => ("127.0.0.1", 1434),
    }
}

fn db_host_mssql_2019() -> (&'static str, usize) {
    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mssql-2019", 1433),
        Err(_) => ("127.0.0.1", 1433),
    }
}

pub fn postgres_9_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "postgresql"
            url = "{}"
            default = true
        }}
    "#,
        postgres_9_url(db_name)
    )
}

pub fn pgbouncer_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "postgresql"
            url = "{}"
            default = true
        }}
    "#,
        pgbouncer_url(db_name)
    )
}

pub fn postgres_10_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "postgresql"
            url = "{}"
            default = true
        }}
    "#,
        postgres_10_url(db_name)
    )
}

pub fn postgres_11_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "postgresql"
            url = "{}"
            default = true
        }}
    "#,
        postgres_11_url(db_name)
    )
}

pub fn postgres_12_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "postgresql"
            url = "{}"
            default = true
        }}
    "#,
        postgres_12_url(db_name)
    )
}

pub fn postgres_13_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "postgresql"
            url = "{}"
            default = true
        }}
    "#,
        postgres_13_url(db_name)
    )
}

pub fn mysql_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "mysql"
            url = "{}"
            default = true
        }}
    "#,
        mysql_url(db_name)
    )
}

pub fn mysql_8_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "mysql"
            url = "{}"
            default = true
        }}
    "#,
        mysql_8_url(db_name)
    )
}

pub fn mysql_5_6_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "mysql"
            url = "{}"
            default = true
        }}
    "#,
        mysql_5_6_url(db_name)
    )
}

pub fn mariadb_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "mysql"
            url = "{}"
            default = true
        }}
    "#,
        mariadb_url(db_name),
    )
}

pub fn sqlite_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "sqlite"
            url = "file:{}"
            default = true
        }}
    "#,
        sqlite_test_file(db_name)
    )
}

pub fn mssql_2017_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "sqlserver"
            url = "{}"
            default = true
        }}
    "#,
        mssql_2017_url(db_name),
    )
}

pub fn mssql_2019_test_config(db_name: &str) -> String {
    format!(
        r#"
        datasource my_db {{
            provider = "sqlserver"
            url = "{}"
            default = true
        }}
    "#,
        mssql_2019_url(db_name),
    )
}

/// The maximum length of identifiers on mysql is 64 bytes.
///
/// Source: https://dev.mysql.com/doc/mysql-reslimits-excerpt/5.5/en/identifier-length.html
pub fn mysql_safe_identifier(identifier: &str) -> &str {
    if identifier.len() < 64 {
        identifier
    } else {
        identifier.get(0..63).expect("mysql identifier truncation")
    }
}

fn fetch_db_name<'a>(url: &'a Url, default: &'static str) -> &'a str {
    match url.path_segments() {
        Some(mut segments) => segments.next().unwrap_or(default),
        None => default,
    }
}

pub async fn create_mysql_database(original_url: &Url) -> Result<Quaint, AnyError> {
    let mut mysql_db_url = original_url.clone();
    mysql_db_url.set_path("/mysql");

    let db_name = fetch_db_name(&original_url, "mysql");

    debug_assert!(!db_name.is_empty());
    debug_assert!(
        db_name.len() < 64,
        "db_name should be less than 64 characters, got {:?}",
        db_name.len()
    );

    let conn = Quaint::new(&mysql_db_url.to_string()).await?;

    let drop = format!(
        r#"
        DROP DATABASE IF EXISTS `{db_name}`;
        "#,
        db_name = db_name,
    );

    let recreate = format!(
        r#"
        CREATE DATABASE `{db_name}`;
        "#,
        db_name = db_name,
    );

    // The two commands have to be run separately on mariadb.
    conn.raw_cmd(&drop).await?;
    conn.raw_cmd(&recreate).await?;

    Ok(Quaint::new(&original_url.to_string()).await?)
}

pub async fn create_postgres_database(original_url: &Url) -> Result<Quaint, AnyError> {
    let mut url = original_url.clone();
    url.set_path("/postgres");

    let db_name = fetch_db_name(&original_url, "postgres");

    let drop = format!(
        r#"
        DROP DATABASE IF EXISTS "{db_name}";
        "#,
        db_name = db_name,
    );

    let recreate = format!(
        r#"
        CREATE DATABASE "{db_name}";
        "#,
        db_name = db_name,
    );

    let conn = Quaint::new(url.as_str()).await?;

    // The two commands have to be run separately on postgres.
    conn.raw_cmd(&drop).await?;
    conn.raw_cmd(&recreate).await?;

    let conn = Quaint::new(&original_url.to_string()).await?;

    conn.raw_cmd("CREATE SCHEMA \"prisma-tests\"").await?;

    Ok(conn)
}

/// Create an MSSQL database from a JDBC connection string..
pub async fn create_mssql_database(jdbc_string: &str) -> Result<Quaint, AnyError> {
    let mut conn = connection_string::JdbcString::from_str(jdbc_string)?;

    let params = conn.properties_mut();
    match params.remove("database") {
        Some(ref db_name) if db_name != "master" => {
            params.insert("database".into(), "master".into());
            let conn = Quaint::new(conn.to_string()).await?;
            conn.raw_cmd(&format!("DROP DATABASE IF EXISTS {}", db_name)).await?;
            conn.raw_cmd(&format!("CREATE DATABASE {}", db_name)).await?;
        }
        _ => (),
    }

    let conn = Quaint::new(jdbc_string).await?;
    conn.query_raw("select db_name() as name", &[]).await?;

    Ok(conn)
}
