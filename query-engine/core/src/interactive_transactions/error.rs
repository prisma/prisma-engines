use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum TransactionError {
    #[error("Unable to start a transaction in the given time.")]
    AcquisitionTimeout,

    #[error("Attempted to start a transaction inside of a transaction.")]
    AlreadyStarted,

    #[error("Transaction not found.")]
    NotFound,

    #[error("Transaction already closed: {reason}.")]
    Closed { reason: String },

    #[error("Unexpected response: {reason}.")]
    Unknown { reason: String },
}
