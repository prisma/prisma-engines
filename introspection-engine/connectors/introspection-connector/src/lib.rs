mod error;

use datamodel::Datamodel;
use error::ConnectorError;

pub type ConnectorResult<T> = Result<T, ConnectorError>;

pub trait IntrospectionConnector: Send + Sync + 'static {
    fn list_databases(&self) -> ConnectorResult<Vec<String>>;

    fn introspect(&self, database: &str) -> ConnectorResult<Datamodel>;
}
