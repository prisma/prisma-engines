use derive_more::Display;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionOptions {
    /// Maximum wait time for tx acquisition in milliseconds.
    #[serde(rename = "max_wait")]
    pub max_acquisition_millis: u64,

    /// Time in milliseconds after which the transaction rolls back automatically.
    #[serde(rename = "timeout")]
    pub valid_for_millis: u64,

    /// Isolation level to use for the transaction.
    pub isolation_level: Option<String>,

    /// An optional pre-defined transaction id. Some value might be provided in case we want to generate
    /// a new id at the beginning of the transaction
    #[serde(skip)]
    pub new_tx_id: Option<TxId>,
}

impl TransactionOptions {
    pub fn new(max_acquisition_millis: u64, valid_for_millis: u64, isolation_level: Option<String>) -> Self {
        Self {
            max_acquisition_millis,
            valid_for_millis,
            isolation_level,
            new_tx_id: None,
        }
    }

    /// Generates a new transaction id before the transaction is started and returns a modified version
    /// of self with the new predefined_id set.
    pub fn with_new_transaction_id(mut self) -> Self {
        let tx_id = TxId::default();
        self.new_tx_id = Some(tx_id.clone());
        self
    }
}
