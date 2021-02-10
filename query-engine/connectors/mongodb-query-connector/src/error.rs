use connector_interface::error::ConnectorError;
use mongodb::error::Error as DriverError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MongoError {
    #[error("Unsupported MongoDB feature: {0}.")]
    Unsupported(String),

    #[error("Failed to convert '{}' to '{}'.", from, to)]
    ConversionError { from: String, to: String },
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
