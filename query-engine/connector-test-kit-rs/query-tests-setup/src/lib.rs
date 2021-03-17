mod config;
mod connector_tag;
mod error;
mod runner;

pub use config::*;
pub use connector_tag::*;
pub use error::*;
pub use runner::*;

use lazy_static::lazy_static;
use serde_json::Value;

pub type TestResult<T> = Result<T, TestError>;

lazy_static! {
    pub static ref CONFIG: TestConfig = TestConfig::load().unwrap();
}

// todo
pub struct QueryResult {
    _json: Value,
}

impl QueryResult {
    pub fn assert_failure(&self, _err_code: usize, _msg_contains: Option<String>) {
        todo!()
    }
}

impl ToString for QueryResult {
    fn to_string(&self) -> String {
        todo!()
    }
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
