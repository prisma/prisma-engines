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

use std::{borrow::Cow, collections::BTreeMap, str::FromStr};

pub use crate::connectors::Features;
use crate::connectors::Tags;
use connection_string::JdbcString;
use enumflags2::BitFlags;
use once_cell::sync::Lazy;
use quaint::{prelude::Queryable, single::Quaint};
use url::Url;

type AnyError = Box<dyn std::error::Error + Send + Sync + 'static>;

const SCHEMA_NAME: &str = "prisma-tests";

pub struct TestApiArgs {
    pub connector_tags: BitFlags<Tags>,
    pub test_function_name: &'static str,
    pub test_features: BitFlags<Features>,
    pub url_fn: &'static (dyn Fn(&str) -> String + Send + Sync),
    pub provider: &'static str,
}

macro_rules! test_api_constructors {
    ($(($connector:ident, $provider:literal),)*) => {
        impl TestApiArgs {
            $(
                pub fn $connector(name: &'static str, tags: u16, features: u8) -> Self {
                    let tags: BitFlags<Tags> = BitFlags::from_bits(tags).unwrap();
                    let features: BitFlags<Features> = BitFlags::from_bits(features).unwrap();

                    TestApiArgs {
                        connector_tags: tags,
                        test_function_name: name,
                        test_features: features,
                        url_fn: &urls::$connector,
                        provider: $provider,
                    }
                }
            )*
        }
    }
}

test_api_constructors!(
    (mssql_2017, "sqlserver"),
    (mssql_2019, "sqlserver"),
    (mysql_5_6, "mysql"),
    (mysql_5_7, "mysql"),
    (mysql_8, "mysql"),
    (mysql_mariadb, "mysql"),
    (postgres9, "postgresql"),
    (postgres10, "postgresql"),
    (postgres11, "postgresql"),
    (postgres12, "postgresql"),
    (postgres13, "postgresql"),
    (sqlite, "sqlite"),
);

impl TestApiArgs {
    pub fn datasource_block(&self, url: &str) -> String {
        format!(
            "datasource db {{\nprovider = \"{provider}\"\nurl = \"{url}\"\ndefault = true\n}}\n\n",
            provider = self.provider,
            url = url
        )
    }
}

mod urls {
    pub use super::mariadb_url as mysql_mariadb;
    pub use super::mssql_2017_url as mssql_2017;
    pub use super::mssql_2019_url as mssql_2019;
    pub use super::mysql_5_6_url as mysql_5_6;
    pub use super::mysql_5_7_url as mysql_5_7;
    pub use super::mysql_8_url as mysql_8;
    pub use super::postgres_10_url as postgres10;
    pub use super::postgres_11_url as postgres11;
    pub use super::postgres_12_url as postgres12;
    pub use super::postgres_13_url as postgres13;
    pub use super::postgres_9_url as postgres9;
    pub use super::sqlite_test_url as sqlite;
}

pub fn sqlite_test_url(db_name: &str) -> String {
    std::env::var("SQLITE_TEST_URL").unwrap_or_else(|_| format!("file:{}", sqlite_test_file(db_name)))
}

pub fn sqlite_test_file(db_name: &str) -> String {
    static SERVER_ROOT: Lazy<std::path::PathBuf> = Lazy::new(|| {
        std::env::var("SERVER_ROOT")
            .map(|root| std::path::Path::new(&root).join("db"))
            .unwrap_or_else(|_| {
                let dir = std::env::temp_dir().join("prisma_tests_server_root");
                let path = dir.to_string_lossy().into_owned();

                std::fs::create_dir_all(&path).expect("failed to create SERVER_ROOT directory");

                path.into()
            })
    });

    let file_path = SERVER_ROOT.join(db_name);

    // Truncate the file.
    std::fs::File::create(&file_path).expect("Failed to create or truncate SQLite database.");

    file_path.to_string_lossy().into_owned()
}

enum TestDb<'a> {
    Schema(&'a str),
    Database(&'a str),
}

fn url_from_env(env_var: &str, db: TestDb<'_>) -> Option<String> {
    std::env::var(env_var).ok().map(|url| {
        let mut url = Url::parse(&url).unwrap();

        match db {
            TestDb::Schema(schema) => {
                let mut params: BTreeMap<String, String> =
                    url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

                params.insert("schema".into(), schema.into());
                url.query_pairs_mut().clear();

                for (k, v) in params.into_iter() {
                    url.query_pairs_mut().append_pair(&k, &v);
                }
            }
            TestDb::Database(db) => {
                url.set_path(db);
            }
        }

        url.to_string()
    })
}

fn jdbc_from_env(env_var: &str, schema: &str) -> Option<String> {
    std::env::var(env_var).ok().and_then(|url| {
        let mut conn_str = JdbcString::from_str(&url).ok()?;
        conn_str.properties_mut().insert("schema".into(), schema.into());
        Some(conn_str.to_string())
    })
}

pub fn postgres_9_url(db_name: &str) -> String {
    url_from_env("POSTGRES_9_TEST_URL", TestDb::Schema(db_name)).unwrap_or_else(|| {
        let (host, port) = db_host_and_port_postgres_9();

        format!(
            "postgresql://postgres:prisma@{}:{}/{}?schema={}&statement_cache_size=0&socket_timeout=60",
            host, port, db_name, SCHEMA_NAME
        )
    })
}

pub fn pgbouncer_url(db_name: &str) -> String {
    url_from_env("PGBOUNCER_TEST_URL", TestDb::Schema(db_name)).unwrap_or_else(|| {
        let (host, port) = db_host_and_port_for_pgbouncer();

        format!(
            "postgresql://postgres:prisma@{}:{}/{}?schema={}&pgbouncer=true&socket_timeout=60",
            host, port, db_name, SCHEMA_NAME
        )
    })
}

pub fn postgres_10_url(db_name: &str) -> String {
    url_from_env("POSTGRES_10_TEST_URL", TestDb::Schema(db_name)).unwrap_or_else(|| {
        let (host, port) = db_host_and_port_postgres_10();

        format!(
            "postgresql://postgres:prisma@{}:{}/{}?schema={}&statement_cache_size=0&socket_timeout=60",
            host, port, db_name, SCHEMA_NAME
        )
    })
}

pub fn postgres_11_url(db_name: &str) -> String {
    url_from_env("POSTGRES_11_TEST_URL", TestDb::Schema(db_name)).unwrap_or_else(|| {
        let (host, port) = db_host_and_port_postgres_11();

        format!(
            "postgresql://postgres:prisma@{}:{}/{}?schema={}&statement_cache_size=0&socket_timeout=60",
            host, port, db_name, SCHEMA_NAME
        )
    })
}

pub fn postgres_12_url(db_name: &str) -> String {
    url_from_env("POSTGRES_12_TEST_URL", TestDb::Schema(db_name)).unwrap_or_else(|| {
        let (host, port) = db_host_and_port_postgres_12();

        format!(
            "postgresql://postgres:prisma@{}:{}/{}?schema={}&statement_cache_size=0&socket_timeout=60",
            host, port, db_name, SCHEMA_NAME
        )
    })
}

pub fn postgres_13_url(db_name: &str) -> String {
    url_from_env("POSTGRES_13_TEST_URL", TestDb::Schema(db_name)).unwrap_or_else(|| {
        let (host, port) = db_host_and_port_postgres_13();

        format!(
            "postgresql://postgres:prisma@{}:{}/{}?schema={}&statement_cache_size=0&socket_timeout=60",
            host, port, db_name, SCHEMA_NAME
        )
    })
}

pub fn mysql_5_7_url(db_name: &str) -> String {
    url_from_env("MYSQL_5_7_TEST_URL", TestDb::Database(db_name)).unwrap_or_else(|| {
        let db_name = mysql_safe_identifier(db_name);
        let (host, port) = db_host_and_port_mysql_5_7();

        format!(
            "mysql://root:prisma@{host}:{port}/{db_name}?connect_timeout=20&socket_timeout=60",
            host = host,
            port = port,
            db_name = db_name,
        )
    })
}

pub fn mysql_8_url(db_name: &str) -> String {
    url_from_env("MYSQL_8_TEST_URL", TestDb::Database(db_name)).unwrap_or_else(|| {
        let (host, port) = db_host_and_port_mysql_8_0();

        // maximum length of identifiers on mysql
        let db_name = mysql_safe_identifier(db_name);

        format!(
            "mysql://root:prisma@{host}:{port}{maybe_slash}{db_name}?connect_timeout=20&socket_timeout=60",
            maybe_slash = if db_name.is_empty() { "" } else { "/" },
            host = host,
            port = port,
            db_name = db_name,
        )
    })
}

pub fn mysql_5_6_url(db_name: &str) -> String {
    url_from_env("MYSQL_5_6_TEST_URL", TestDb::Database(db_name)).unwrap_or_else(|| {
        let (host, port) = db_host_and_port_mysql_5_6();

        // maximum length of identifiers on mysql
        let db_name = mysql_safe_identifier(db_name);

        format!(
            "mysql://test:test@{host}:{port}/{db_name}?connect_timeout=20&socket_timeout=60",
            host = host,
            port = port,
            db_name = db_name,
        )
    })
}

pub fn mariadb_url(db_name: &str) -> String {
    url_from_env("MARIADB_TEST_URL", TestDb::Database(db_name)).unwrap_or_else(|| {
        let (host, port) = db_host_and_port_mariadb();

        // maximum length of identifiers on mysql
        let db_name = mysql_safe_identifier(db_name);

        format!(
            "mysql://root:prisma@{host}:{port}/{db_name}?connect_timeout=20&socket_timeout=60",
            host = host,
            port = port,
            db_name = db_name,
        )
    })
}

pub fn mssql_2017_url(schema_name: &str) -> String {
    jdbc_from_env("MSSQL_2017_TEST_URL", schema_name).unwrap_or_else(|| {
        let (host, port) = db_host_mssql_2017();

        format!(
            "sqlserver://{host}:{port};database=master;schema={schema_name};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;socket_timeout=60;isolationLevel=READ UNCOMMITTED",
            schema_name = schema_name,
            host = host,
            port = port,
        )
    })
}

pub fn mssql_2019_url(schema_name: &str) -> String {
    jdbc_from_env("MSSQL_2019_TEST_URL", schema_name).unwrap_or_else(|| {
        let (host, port) = db_host_and_port_mssql_2019();

        format!(
            "sqlserver://{host}:{port};database=master;schema={schema_name};user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;socket_timeout=60;isolationLevel=READ UNCOMMITTED",
            schema_name = schema_name,
            host = host,
            port = port,
        )
    })
}

fn db_host_and_port_postgres_9() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("POSTGRES_9_TEST_URL")
        .ok()
        .and_then(|s| Url::parse(&s).ok())
    {
        let host = var
            .host()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(5432) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-postgres-9".into(), 5432),
        Err(_) => ("127.0.0.1".into(), 5431),
    }
}

fn db_host_and_port_postgres_10() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("POSTGRES_10_TEST_URL")
        .ok()
        .and_then(|s| Url::parse(&s).ok())
    {
        let host = var
            .host()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(5432) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-postgres-10".into(), 5432),
        Err(_) => ("127.0.0.1".into(), 5432),
    }
}

fn db_host_and_port_postgres_11() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("POSTGRES_11_TEST_URL")
        .ok()
        .and_then(|s| Url::parse(&s).ok())
    {
        let host = var
            .host()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(5432) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-postgres-11".into(), 5432),
        Err(_) => ("127.0.0.1".into(), 5433),
    }
}

fn db_host_and_port_for_pgbouncer() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("PGBOUNCER_TEST_URL")
        .ok()
        .and_then(|s| Url::parse(&s).ok())
    {
        let host = var
            .host()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(5432) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-pgbouncer".into(), 6432),
        Err(_) => ("127.0.0.1".into(), 6432),
    }
}

pub fn db_host_and_port_postgres_12() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("POSTGRES_12_TEST_URL")
        .ok()
        .and_then(|s| Url::parse(&s).ok())
    {
        let host = var
            .host()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(5432) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-postgres-12".into(), 5432),
        Err(_) => ("127.0.0.1".into(), 5434),
    }
}

fn db_host_and_port_postgres_13() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("POSTGRES_13_TEST_URL")
        .ok()
        .and_then(|s| Url::parse(&s).ok())
    {
        let host = var
            .host()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(5432) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-postgres-13".into(), 5432),
        Err(_) => ("127.0.0.1".into(), 5435),
    }
}

pub fn db_host_and_port_mysql_8_0() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("MYSQL_8_TEST_URL").ok().and_then(|s| Url::parse(&s).ok()) {
        let host = var
            .host()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(3306) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mysql-8-0".into(), 3306),
        Err(_) => ("127.0.0.1".into(), 3307),
    }
}

fn db_host_and_port_mysql_5_6() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("MYSQL_5_6_TEST_URL")
        .ok()
        .and_then(|s| Url::parse(&s).ok())
    {
        let host = var
            .host()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(3306) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mysql-5-6".into(), 3306),
        Err(_) => ("127.0.0.1".into(), 3309),
    }
}

pub fn db_host_and_port_mysql_5_7() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("MYSQL_5_7_TEST_URL")
        .ok()
        .and_then(|s| Url::parse(&s).ok())
    {
        let host = var
            .host()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(3306) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mysql-5-7".into(), 3306),
        Err(_) => ("127.0.0.1".into(), 3306),
    }
}

fn db_host_and_port_mariadb() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("MARIADB_TEST_URL").ok().and_then(|s| Url::parse(&s).ok()) {
        let host = var
            .host()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(3306) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mariadb".into(), 3306),
        Err(_) => ("127.0.0.1".into(), 3308),
    }
}

fn db_host_mssql_2017() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("MSSQL_2017_TEST_URL")
        .ok()
        .and_then(|s| JdbcString::from_str(&s).ok())
    {
        let host = var
            .server_name()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(1433) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mssql-2017".into(), 1433),
        Err(_) => ("127.0.0.1".into(), 1434),
    }
}

pub fn db_host_and_port_mssql_2019() -> (Cow<'static, str>, usize) {
    if let Some(var) = std::env::var("MSSQL_2019_TEST_URL")
        .ok()
        .and_then(|s| JdbcString::from_str(&s).ok())
    {
        let host = var
            .server_name()
            .map(|s| Cow::from(s.to_string()))
            .unwrap_or_else(|| Cow::from("localhost"));

        let port = var.port().unwrap_or(1433) as usize;

        return (host, port);
    }

    match std::env::var("IS_BUILDKITE") {
        Ok(_) => ("test-db-mssql-2019".into(), 1433),
        Err(_) => ("127.0.0.1".into(), 1433),
    }
}

/// The maximum length of identifiers on mysql is 64 bytes. (and 60 on vitess)
///
/// Source: https://dev.mysql.com/doc/mysql-reslimits-excerpt/5.5/en/identifier-length.html
pub fn mysql_safe_identifier(identifier: &str) -> &str {
    if identifier.len() < 60 {
        identifier
    } else {
        identifier.get(0..59).expect("mysql identifier truncation")
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
        db_name.len() < 60,
        "db_name should be less than 60 characters, got {:?}",
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
