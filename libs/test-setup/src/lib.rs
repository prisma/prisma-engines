#![deny(unsafe_code, rust_2018_idioms)]

//! This crate contains constants and utilities that are useful for writing tests across the
//! engines.

/// Tokio test runtime utils.
pub mod runtime;

mod capabilities;
mod logging;
mod tags;

mod mssql;
mod mysql;
mod postgres;
mod sqlite;

pub use capabilities::Capabilities;
pub use enumflags2::BitFlags;
pub use mssql::{init_mssql_database, reset_schema as reset_mssql_schema};
pub use mysql::{create_mysql_database, mysql_safe_identifier};
pub use postgres::create_postgres_database;
pub use sqlite::sqlite_test_url;
pub use tags::Tags;

use once_cell::sync::Lazy;
use std::io::Write as _;

type AnyError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug)]
struct DbUnderTest {
    capabilities: BitFlags<Capabilities>,
    database_url: String,
    shadow_database_url: Option<String>,
    provider: &'static str,
    tags: BitFlags<Tags>,
}

const MISSING_TEST_DATABASE_URL_MSG: &str = r#"
Missing TEST_DATABASE URL from environment.

If you are developing with the docker-compose based setup, you can find the environment variables under .test_database_urls at the project root.

Example usage:

source .test_database_urls/mysql_5_6
"#;

static TAGS: Lazy<Result<DbUnderTest, String>> = Lazy::new(|| {
    let database_url = if std::env::var("IS_BUILDKITE").is_ok() {
        "sqlite".to_owned()
    } else {
        std::env::var("TEST_DATABASE_URL").map_err(|_| MISSING_TEST_DATABASE_URL_MSG.to_owned())?
    };
    let shadow_database_url = std::env::var("TEST_SHADOW_DATABASE_URL").ok();
    let prefix = database_url
        .find(':')
        .map(|prefix_end| &database_url[..prefix_end])
        .unwrap_or_else(|| database_url.as_str());

    logging::init_logger();

    match prefix {
        "file" | "sqlite" => Ok(DbUnderTest {
            database_url,
            tags: Tags::Sqlite.into(),
            capabilities: Capabilities::CreateDatabase.into(),
            provider: "sqlite",
            shadow_database_url,
        }),
        "mysql" => {
            let tags = mysql::get_mysql_tags(&database_url)?;
            let mut capabilities = Capabilities::Enums | Capabilities::Json | Capabilities::Decimal;

            if tags.contains(Tags::Vitess) {
                capabilities |= Capabilities::CreateDatabase;
            }

            Ok(DbUnderTest {
                tags,
                database_url,
                capabilities,
                provider: "mysql",
                shadow_database_url,
            })
        }
        "postgresql" | "postgres" => Ok(DbUnderTest {
            tags: postgres::get_postgres_tags(&database_url)?,
            database_url,
            capabilities: Capabilities::Enums
                | Capabilities::Json
                | Capabilities::ScalarLists
                | Capabilities::CreateDatabase
                | Capabilities::Decimal,
            provider: "postgresql",
            shadow_database_url,
        }),
        "sqlserver" => Ok(DbUnderTest {
            tags: mssql::get_mssql_tags(&database_url)?,
            database_url,
            capabilities: (Capabilities::CreateDatabase | Capabilities::Decimal).into(),
            provider: "sqlserver",
            shadow_database_url,
        }),
        _ => Err("Unknown database URL".into()),
    }
});

fn db_under_test() -> &'static DbUnderTest {
    match TAGS.as_ref() {
        Ok(db) => db,
        Err(explanation) => {
            let stderr = std::io::stderr();

            let mut sink = stderr.lock();
            sink.write_all(explanation.as_bytes()).unwrap();
            sink.write_all(b"\n").unwrap();

            std::process::exit(1)
        }
    }
}

pub fn should_skip_test(
    args: &TestApiArgs,
    include_tagged: BitFlags<Tags>,
    exclude_tags: BitFlags<Tags>,
    capabilities: BitFlags<Capabilities>,
) -> bool {
    if !capabilities.is_empty() && !args.capabilities().contains(capabilities) {
        println!("Test skipped");
        return true;
    }

    if !include_tagged.is_empty() && !include_tagged.intersects(args.tags()) {
        println!("Test skipped");
        return true;
    }

    if exclude_tags.intersects(args.tags()) {
        println!("Test skipped");
        return true;
    }

    false
}

#[derive(Debug)]
pub struct TestApiArgs {
    test_function_name: &'static str,
    db: &'static DbUnderTest,
}

impl TestApiArgs {
    pub fn new(test_function_name: &'static str) -> Self {
        TestApiArgs {
            test_function_name,
            db: db_under_test(),
        }
    }

    pub fn test_function_name(&self) -> &'static str {
        self.test_function_name
    }

    pub fn capabilities(&self) -> BitFlags<Capabilities> {
        self.db.capabilities
    }

    pub fn database_url(&self) -> &str {
        self.db.database_url.as_str()
    }

    pub fn datasource_block(&self, url: &str) -> String {
        format!(
            "datasource db {{\nprovider = \"{provider}\"\nurl = \"{url}\"\ndefault = true\n}}\n\n",
            provider = self.db.provider,
            url = url
        )
    }

    pub fn provider(&self) -> &str {
        self.db.provider
    }

    pub fn shadow_database_url(&self) -> Option<&'static str> {
        self.db.shadow_database_url.as_deref()
    }

    pub fn tags(&self) -> BitFlags<Tags> {
        self.db.tags
    }
}
