use migration_connector::{ConnectorError, ConnectorResult};
use mongodb::error::Error as MongoError;

pub trait IntoConnectorResult<T> {
    fn into_connector_result(self) -> ConnectorResult<T>;
}

impl<T> IntoConnectorResult<T> for std::result::Result<T, MongoError> {
    fn into_connector_result(self) -> ConnectorResult<T> {
        self.map_err(|err| ConnectorError::from_source(err, "MongoDB error"))
    }
}
