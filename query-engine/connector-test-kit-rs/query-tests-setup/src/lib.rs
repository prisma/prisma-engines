mod config;
mod connector_tag;
mod error;
mod query_result;
mod runner;
mod schema_rendering;
mod templating;

pub use config::*;
pub use connector_tag::*;
pub use error::*;
pub use query_result::*;
pub use runner::*;
pub use templating::*;

use lazy_static::lazy_static;
use tokio::runtime::Builder;

pub type TestResult<T> = Result<T, TestError>;

lazy_static! {
    /// Test configuration, loaded once at runtime.
    pub static ref CONFIG: TestConfig = TestConfig::load().unwrap();
}

/// Render the complete datamodel with all bells and whistles.
pub fn render_test_datamodel(config: &TestConfig, test_database: &str, template: String) -> String {
    let tag = config.test_connector_tag().unwrap();
    let datasource_with_generator = format!(
        r#"
      datasource test {{
        provider = "{}"
        url = "{}"
      }}

      generator client {{
        provider = "prisma-client-js"
        previewFeatures = ["microsoftSqlServer"]
      }}
    "#,
        tag.datamodel_provider(),
        tag.connection_string(test_database, config.is_ci())
    );

    let models = tag.render_datamodel(template);
    format!("{}\n\n{}", datasource_with_generator, models)
}

/// Teardown & setup of everything as defined in the passed datamodel.
pub async fn setup_project(datamodel: &str) -> TestResult<()> {
    Ok(migration_core::qe_setup(datamodel).await?)
}

/// Helper method to allow a sync shell function to run the async test blocks.
pub fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}
