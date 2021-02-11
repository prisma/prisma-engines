#![deny(missing_docs)]

//! A TestApi that is initialized without IO or async code and can instantiate
//! multiple migration engines.

use migration_core::GenericApi;
use quaint::single::Quaint;
use tempfile::TempDir;
use test_setup::{connectors::Tags, TestApiArgs};

/// The multi-engine test API.
pub struct TestApi {
    args: TestApiArgs,
    source: String,
    url: String,
}

impl TestApi {
    /// Initializer, called by the test macros.
    pub async fn new(args: TestApiArgs) -> Self {
        let connection_string = (args.url_fn)(args.test_function_name);
        let source = args.datasource_block(&connection_string);

        TestApi {
            args,
            url: connection_string,
            source,
        }
    }

    /// Create a temporary directory to serve as a test migrations directory.
    pub fn create_migrations_directory(&self) -> anyhow::Result<TempDir> {
        Ok(tempfile::tempdir()?)
    }

    /// Instantiate a new migration engine.
    pub async fn new_engine(&self) -> anyhow::Result<Box<dyn GenericApi>> {
        Ok(migration_core::migration_api(&self.source).await?)
    }

    /// Initialize the database.
    pub async fn initialize(&self) -> anyhow::Result<()> {
        if self.args.connector_tags.contains(Tags::Postgres) {
            test_setup::create_postgres_database(&self.url.parse()?).await.unwrap();
        } else if self.args.connector_tags.contains(Tags::Mysql) {
            test_setup::create_mysql_database(&self.url.parse()?).await.unwrap();
        } else if self.args.connector_tags.contains(Tags::Mssql) {
            let conn = Quaint::new(&self.url).await.unwrap();
            test_setup::connectors::mssql::reset_schema(&conn, self.args.test_function_name)
                .await
                .unwrap();
        }

        Ok(())
    }
}
