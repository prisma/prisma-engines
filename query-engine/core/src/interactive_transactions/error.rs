use thiserror::Error;

use crate::{
    response_ir::{Item, Map},
    CoreError,
};

#[derive(Debug, Error, PartialEq)]
pub enum TransactionError {
    #[error("Unable to start a transaction in the given time.")]
    AcquisitionTimeout,

    #[error("Attempted to start a transaction inside of a transaction.")]
    AlreadyStarted,

    #[error("Transaction not found. Transaction ID is invalid, refers to an old closed transaction Prisma doesn't have information about anymore, or was obtained before disconnecting.")]
    NotFound,

    #[error("Transaction already closed: {reason}.")]
    Closed { reason: String },

    #[error("Unexpected response: {reason}.")]
    Unknown { reason: String },
}

#[derive(Debug, serde::Serialize)]
pub struct ExtendedTransactionUserFacingError {
    #[serde(flatten)]
    user_facing_error: user_facing_errors::Error,

    #[serde(skip_serializing_if = "indexmap::IndexMap::is_empty")]
    extensions: Map,
}

impl ExtendedTransactionUserFacingError {
    pub fn set_extension(&mut self, key: String, val: serde_json::Value) {
        self.extensions.entry(key).or_insert(Item::Json(val));
    }
}

impl From<CoreError> for ExtendedTransactionUserFacingError {
    fn from(error: CoreError) -> Self {
        ExtendedTransactionUserFacingError {
            user_facing_error: error.into(),
            extensions: Default::default(),
        }
    }
}
