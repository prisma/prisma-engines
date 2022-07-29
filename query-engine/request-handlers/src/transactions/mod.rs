use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TxInput {
    /// Maximum wait time in milliseconds.
    pub max_wait: u64,

    /// Time in milliseconds after which the transaction rolls back automatically.
    pub timeout: u64,

    /// Isolation level to use for the transaction.
    pub isolation_level: Option<String>,
}
