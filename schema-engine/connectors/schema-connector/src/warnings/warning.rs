use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A warning, spawned from an introspection run.
#[derive(Serialize, Deserialize, Debug)]
pub struct Warning {
    /// A unique indentifying code for the warning.
    pub code: u32,
    /// The warning message.
    pub message: String,
    /// The affected items that triggered this warning.
    pub affected: Value,
}
