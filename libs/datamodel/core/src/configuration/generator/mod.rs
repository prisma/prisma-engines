mod json;
mod loader;
pub use json::*;
pub use loader::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[serde(rename_all = "camelCase")]
#[derive(Debug, Serialize, Deserialize)]
pub struct Generator {
    name: String,
    provider: String,
    output: Option<String>,
    #[serde(default = "Vec::new")]
    binary_targets: Vec<String>,
    // Todo: This is a bad choice, PrismaValue is probably better.
    config: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    documentation: Option<String>,
}
