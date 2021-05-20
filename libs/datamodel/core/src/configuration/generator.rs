use super::StringFromEnvVar;
use crate::common::preview_features::PreviewFeature;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Generator {
    pub name: String,
    pub provider: StringFromEnvVar,
    pub output: Option<StringFromEnvVar>,
    pub config: HashMap<String, String>,

    #[serde(default)]
    pub binary_targets: Vec<String>,

    #[serde(default)]
    pub preview_features: Vec<PreviewFeature>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}
