mod loader;
pub use loader::*;

use serde::Serialize;
use std::collections::HashMap;

#[serde(rename_all = "camelCase")]
#[derive(Debug, Serialize)]
pub struct Generator {
    name: String,
    provider: String,
    output: Option<String>,
    #[serde(default = "Vec::new")]
    binary_targets: Vec<String>,
    #[serde(default = "Vec::new")]
    experimental_features: Vec<String>,
    config: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    documentation: Option<String>,
}
