use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum TransactionError {
    #[error("Unable to start a transaction in the given time.")]
    AcquisitionTimeout,

    #[error("Attempted to start a transaction inside of a transaction.")]
    AlreadyStarted,

    #[error(
        "Transaction not found. Transaction ID is invalid, refers to an old closed transaction Prisma doesn't have information about anymore, or was obtained before disconnecting."
    )]
    NotFound,

    #[error("Transaction already closed: {reason}.")]
    Closed { reason: String },

    #[error("Unexpected response: {reason}.")]
    Unknown { reason: String },
}
