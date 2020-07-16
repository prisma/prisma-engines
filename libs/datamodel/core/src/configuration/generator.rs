use serde::Serialize;
use std::collections::HashMap;

#[serde(rename_all = "camelCase")]
#[derive(Debug, Serialize)]
pub struct Generator {
    pub name: String,
    pub provider: String,
    pub output: Option<String>,
    #[serde(default = "Vec::new")]
    pub binary_targets: Vec<String>,
    #[serde(default = "Vec::new")]
    pub experimental_features: Vec<String>,
    pub config: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}
