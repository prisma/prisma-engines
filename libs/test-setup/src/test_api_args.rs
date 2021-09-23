use crate::{logging, mssql, mysql, postgres, Capabilities, Tags};
use enumflags2::BitFlags;
use once_cell::sync::Lazy;
use quaint::single::Quaint;
use std::{fmt::Display, io::Write as _};

#[derive(Debug)]
pub(crate) struct DbUnderTest {
    capabilities: BitFlags<Capabilities>,
    database_url: String,
    shadow_database_url: Option<String>,
    provider: &'static str,
    tags: BitFlags<Tags>,
}

const MISSING_TEST_DATABASE_URL_MSG: &str = r#"
Missing TEST_DATABASE_URL from environment.

If you are developing with the docker-compose based setup, you can find the environment variables under .test_database_urls at the project root.

Example usage:

source .test_database_urls/mysql_5_6
"#;

static DB_UNDER_TEST: Lazy<Result<DbUnderTest, String>> = Lazy::new(|| {
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
            let mut capabilities = Capabilities::Enums | Capabilities::Json;

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
                | Capabilities::CreateDatabase,
            provider: "postgresql",
            shadow_database_url,
        }),
        "sqlserver" => Ok(DbUnderTest {
            tags: mssql::get_mssql_tags(&database_url)?,
            database_url,
            capabilities: Capabilities::CreateDatabase.into(),
            provider: "sqlserver",
            shadow_database_url,
        }),
        _ => Err("Unknown database URL".into()),
    }
});

/// Crate-public interface to the global test database state.
pub(crate) fn db_under_test() -> &'static DbUnderTest {
    match DB_UNDER_TEST.as_ref() {
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

/// Context for test initialization.
#[derive(Debug)]
pub struct TestApiArgs {
    test_function_name: &'static str,
    preview_features: &'static [&'static str],
    db: &'static DbUnderTest,
}

impl TestApiArgs {
    pub fn new(test_function_name: &'static str, preview_features: &'static [&'static str]) -> Self {
        TestApiArgs {
            test_function_name,
            preview_features,
            db: db_under_test(),
        }
    }

    pub fn preview_features(&self) -> &'static [&'static str] {
        self.preview_features
    }

    pub fn test_function_name(&self) -> &'static str {
        self.test_function_name
    }

    pub fn capabilities(&self) -> BitFlags<Capabilities> {
        self.db.capabilities
    }

    pub async fn create_mssql_database(&self) -> (Quaint, String) {
        mssql::init_mssql_database(self.database_url(), self.test_function_name)
            .await
            .unwrap()
    }

    pub async fn create_mysql_database(&self) -> (&'static str, String) {
        mysql::create_mysql_database(self.database_url(), self.test_function_name)
            .await
            .unwrap()
    }

    pub async fn create_postgres_database(&self) -> (&'static str, Quaint, String) {
        let (q, cs) = postgres::create_postgres_database(self.database_url(), self.test_function_name())
            .await
            .unwrap();
        (self.test_function_name(), q, cs)
    }

    pub fn database_url(&self) -> &str {
        self.db.database_url.as_str()
    }

    pub fn datasource_block<'a>(&'a self, url: &'a str, params: &'a [(&'a str, &'a str)]) -> DatasourceBlock<'a> {
        DatasourceBlock {
            provider: self.db.provider,
            url,
            params,
        }
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

pub struct DatasourceBlock<'a> {
    provider: &'a str,
    url: &'a str,
    params: &'a [(&'a str, &'a str)],
}

impl<'a> DatasourceBlock<'a> {
    pub fn url(&self) -> &str {
        self.url
    }
}

impl Display for DatasourceBlock<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("datasource db {\n    provider = \"")?;
        f.write_str(self.provider)?;
        f.write_str("\"\n    url = \"")?;
        f.write_str(self.url)?;
        f.write_str("\"\n")?;

        for (param_name, param_value) in self.params {
            f.write_str("    ")?;
            f.write_str(param_name)?;
            f.write_str(" = ")?;
            f.write_str(param_value)?;
            f.write_str("\n")?;
        }

        f.write_str("}")
    }
}
