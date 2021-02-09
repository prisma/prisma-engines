use connector_interface::error::ConnectorError;
use mongodb::error::Error as DriverError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MongoError {
    #[error("Test")]
    Test,
}

impl From<DriverError> for MongoError {
    fn from(err: DriverError) -> Self {
        todo!()
    }
}

impl MongoError {
    pub fn into_connector_error(self) -> ConnectorError {
        todo!()
    }
}
