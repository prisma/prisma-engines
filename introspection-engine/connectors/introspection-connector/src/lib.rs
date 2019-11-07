mod error;

use datamodel::Datamodel;
pub use error::ConnectorError;
use serde::*;

pub type ConnectorResult<T> = Result<T, ConnectorError>;

pub trait IntrospectionConnector: Send + Sync + 'static {
    fn list_databases(&self) -> ConnectorResult<Vec<String>>;
    fn get_metadata(&self, database: &str) -> ConnectorResult<DatabaseMetadata>;

    fn introspect(&self, database: &str) -> ConnectorResult<Datamodel>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseMetadata {
    pub model_count: usize,
    pub size_in_bytes: usize,
}
