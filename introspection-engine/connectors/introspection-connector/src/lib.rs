mod error;

use core::fmt;
use datamodel::Datamodel;
pub use error::{ConnectorError, ErrorKind};
use serde::*;
use serde_json::Value;

pub type ConnectorResult<T> = Result<T, ConnectorError>;

#[async_trait::async_trait]
pub trait IntrospectionConnector: Send + Sync + 'static {
    async fn list_databases(&self) -> ConnectorResult<Vec<String>>;

    async fn get_metadata(&self) -> ConnectorResult<DatabaseMetadata>;

    async fn get_database_description(&self) -> ConnectorResult<String>;

    async fn introspect(&self) -> ConnectorResult<IntrospectionResult>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseMetadata {
    pub table_count: usize,
    pub size_in_bytes: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Version {
    NonPrisma,
    Prisma1,
    Prisma11,
    Prisma2,
}

#[derive(Debug)]
pub struct IntrospectionResult {
    /// Datamodel
    pub datamodel: Datamodel,
    /// warnings
    pub warnings: Vec<Warning>,
    /// version
    pub version: Version,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Warning {
    pub code: i8,
    pub message: String,
    pub affected: Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IntrospectionResultOutput {
    /// Datamodel
    pub datamodel: String,
    /// warnings
    pub warnings: Vec<Warning>,
    /// version
    pub version: Version,
}

impl fmt::Display for IntrospectionResultOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\"datamodel\": \"{}\", \"warnings\": {}}}",
            self.datamodel,
            serde_json::to_string(&self.warnings).unwrap()
        )
    }
}
