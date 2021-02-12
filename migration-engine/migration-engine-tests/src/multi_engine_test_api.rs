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
    connection_string: String,
}

impl TestApi {
    /// Initializer, called by the test macros.
    pub async fn new(args: TestApiArgs) -> Self {
        let connection_string = (args.url_fn)(args.test_function_name);
        let source = args.datasource_block(&connection_string);

        TestApi {
            args,
            connection_string,
            source,
        }
    }

    /// The connection string for the database associated with the test.
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// Create a temporary directory to serve as a test migrations directory.
    pub fn create_migrations_directory(&self) -> anyhow::Result<TempDir> {
        Ok(tempfile::tempdir()?)
    }

    /// Instantiate a new migration engine for the current database.
    pub async fn new_engine(&self) -> anyhow::Result<Box<dyn GenericApi>> {
        self.new_engine_with_datasource(&self.source).await
    }

    /// Instantiate a new migration with the provided connection string.
    pub async fn new_engine_with_connection_string(
        &self,
        connection_string: &str,
    ) -> anyhow::Result<Box<dyn GenericApi>> {
        self.new_engine_with_datasource(&self.args.datasource_block(connection_string))
            .await
    }

    async fn new_engine_with_datasource(&self, datasource: &str) -> anyhow::Result<Box<dyn GenericApi>> {
        Ok(migration_core::migration_api(&datasource).await?)
    }

    /// Initialize the database.
    pub async fn initialize(&self) -> anyhow::Result<Quaint> {
        if self.args.connector_tags.contains(Tags::Postgres) {
            Ok(test_setup::create_postgres_database(&self.connection_string.parse()?)
                .await
                .unwrap())
        } else if self.args.connector_tags.contains(Tags::Mysql) {
            Ok(test_setup::create_mysql_database(&self.connection_string.parse()?)
                .await
                .unwrap())
        } else if self.args.connector_tags.contains(Tags::Mssql) {
            let conn = Quaint::new(&self.connection_string).await?;
            test_setup::connectors::mssql::reset_schema(&conn, self.args.test_function_name).await?;
            Ok(conn)
        } else {
            unreachable!()
        }
    }

    /// The name of the test function, as a string.
    pub fn test_fn_name(&self) -> &str {
        self.args.test_function_name
    }
}
