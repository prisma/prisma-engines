use super::StringFromEnvVar;
use crate::configuration::preview_features::PreviewFeatures;
use serde::Serialize;
use std::collections::HashMap;

#[serde(rename_all = "camelCase")]
#[derive(Debug, Serialize, Clone)]
pub struct Generator {
    pub name: String,
    pub provider: StringFromEnvVar,
    pub output: Option<StringFromEnvVar>,
    #[serde(default = "Vec::new")]
    pub binary_targets: Vec<String>,
    #[serde(default = "Vec::new")]
    pub preview_features: Vec<String>,
    pub config: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

impl PreviewFeatures for Generator {
    fn preview_features(&self) -> &[String] {
        &self.preview_features
    }
}
