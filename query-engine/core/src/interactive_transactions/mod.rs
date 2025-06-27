use derive_more::Display;
use serde::{Deserialize, Serialize};

mod error;
mod manager;
mod transaction;

pub use error::*;

pub(crate) use manager::*;
pub(crate) use transaction::*;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, Display)]
#[display(fmt = "{_0}")]
pub struct TxId(String);

impl Default for TxId {
    fn default() -> Self {
        Self(cuid::cuid2())
    }
}

impl<T> From<T> for TxId
where
    T: Into<String>,
{
    fn from(s: T) -> Self {
        const MINIMUM_TX_ID_LENGTH: usize = 24;

        let contents = s.into();
        // This postcondition is to ensure that the TxId is long enough as to be able to derive
        // a TraceId from it. See `TxTraceExt` trait for more details.
        assert!(
            contents.len() >= MINIMUM_TX_ID_LENGTH,
            "minimum length for a TxId ({}) is {}, but was {}",
            contents,
            MINIMUM_TX_ID_LENGTH,
            contents.len()
        );
        Self(contents)
    }
}
