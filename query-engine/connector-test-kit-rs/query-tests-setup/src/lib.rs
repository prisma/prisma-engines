mod connector_tag;

pub use connector_tag::*;

use serde_json::Value;

// todo
pub struct QueryResult {
    json: Value,
}

impl QueryResult {
    pub fn assert_failure(&self, err_code: usize, msg_contains: Option<String>) {
        todo!()
    }
}

impl ToString for QueryResult {
    fn to_string(&self) -> String {
        todo!()
    }
}

pub enum Runner {
    /// Using the QE crate directly for queries.
    Direct(DirectRunner),

    /// Using a NodeJS runner.
    NApi(NApiRunner),

    /// Using the HTTP bridge
    Binary(BinaryRunner),
}

impl Runner {
    pub fn load() -> Self {
        println!("Totally loaded");
        Self::Direct(DirectRunner {})
    }

    pub fn query<T>(&self, gql: T) -> QueryResult
    where
        T: Into<String>,
    {
        todo!()
    }

    pub fn batch<T>(&self, gql: T) -> QueryResult
    where
        T: Into<String>,
    {
        todo!()
    }
}

pub struct DirectRunner {}
pub struct NApiRunner {}
pub struct BinaryRunner {}

/// Wip, just a collection of env vars we might want.
struct EnvConfig {
    /// MIGRATION_ENGINE_PATH
    migration_engine_path: String,

    /// TEST_RUNNER
    runner: String,
}
