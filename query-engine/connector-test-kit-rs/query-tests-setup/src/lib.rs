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
