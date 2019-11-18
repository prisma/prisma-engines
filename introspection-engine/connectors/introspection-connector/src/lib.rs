mod error;

use datamodel::Datamodel;
pub use error::ConnectorError;
use serde::*;

pub type ConnectorResult<T> = Result<T, ConnectorError>;

#[async_trait::async_trait]
pub trait IntrospectionConnector: Send + Sync + 'static {
    async fn list_databases(&self) -> ConnectorResult<Vec<String>>;

    async fn get_metadata(&self, database: &str) -> ConnectorResult<DatabaseMetadata>;

    async fn introspect(&self, database: &str) -> ConnectorResult<Datamodel>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseMetadata {
    pub table_count: usize,
    pub size_in_bytes: usize,
}
