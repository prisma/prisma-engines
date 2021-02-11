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
        dbg!(&err);
        match err.kind.as_ref() {
            mongodb::error::ErrorKind::AddrParse(_) => todo!(),
            mongodb::error::ErrorKind::ArgumentError { .. } => todo!(),
            mongodb::error::ErrorKind::AuthenticationError { .. } => todo!(),
            mongodb::error::ErrorKind::BsonDecode(_) => todo!(),
            mongodb::error::ErrorKind::BsonEncode(_) => todo!(),
            mongodb::error::ErrorKind::BulkWriteError(_err) => {
                //
                todo!()
            }
            mongodb::error::ErrorKind::CommandError(_) => todo!(),
            mongodb::error::ErrorKind::DnsResolve(_) => todo!(),
            mongodb::error::ErrorKind::InternalError { .. } => todo!(),
            mongodb::error::ErrorKind::InvalidDnsName(_) => todo!(),
            mongodb::error::ErrorKind::InvalidHostname { .. } => todo!(),
            mongodb::error::ErrorKind::Io(_) => todo!(),
            mongodb::error::ErrorKind::NoDnsResults(_) => todo!(),
            mongodb::error::ErrorKind::OperationError { .. } => todo!(),
            mongodb::error::ErrorKind::OutOfRangeError(_) => todo!(),
            mongodb::error::ErrorKind::ParseError { .. } => todo!(),
            mongodb::error::ErrorKind::ConnectionPoolClearedError { .. } => todo!(),
            mongodb::error::ErrorKind::ResponseError { .. } => todo!(),
            mongodb::error::ErrorKind::ServerSelectionError { .. } => todo!(),
            mongodb::error::ErrorKind::SrvLookupError { .. } => todo!(),
            mongodb::error::ErrorKind::TokioTimeoutElapsed(_) => todo!(),
            mongodb::error::ErrorKind::RustlsConfig(_) => todo!(),
            mongodb::error::ErrorKind::TxtLookupError { .. } => todo!(),
            mongodb::error::ErrorKind::WaitQueueTimeoutError { .. } => todo!(),
            mongodb::error::ErrorKind::WriteError(_) => todo!(),
            _ => todo!(),
        }
    }
}

impl MongoError {
    pub fn into_connector_error(self) -> ConnectorError {
        todo!()
    }
}
