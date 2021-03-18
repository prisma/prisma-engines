mod config;
mod connector_tag;
mod error;
mod query_result;
mod runner;

pub use config::*;
pub use connector_tag::*;
pub use error::*;
pub use query_result::*;
pub use runner::*;

use lazy_static::lazy_static;
use tokio::runtime::Builder;

pub type TestResult<T> = Result<T, TestError>;

lazy_static! {
    pub static ref CONFIG: TestConfig = TestConfig::load().unwrap();
}

/// Render the complete datamodel with all bells and whistles.
pub fn render_test_datamodel(config: &TestConfig, suite: &str, template: String) -> String {
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
        tag.connection_string(suite, config.is_ci())
    );

    let models = tag.render_datamodel(template);
    format!("{}\n\n{}", datasource_with_generator, models)
}

pub async fn setup_project(datamodel: &str) -> TestResult<()> {
    Ok(migration_core::qe_setup(datamodel).await?)
}

pub fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}
