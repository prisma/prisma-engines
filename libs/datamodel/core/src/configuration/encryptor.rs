use serde::Serialize;
use std::collections::HashMap;

#[serde(rename_all = "camelCase")]
#[derive(Debug, Serialize, Clone)]
pub struct Encryptor {
    pub name: String,
    pub provider: String,
    pub token: Option<String>,
    pub config: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}
