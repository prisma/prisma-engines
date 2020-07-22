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
    pub preview_features: Vec<String>,
    pub config: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

pub trait PreviewFeatures {
    fn has_preview_feature(&self, feature: &str) -> bool;
}

impl PreviewFeatures for Generator {
    fn has_preview_feature(&self, feature: &str) -> bool {
        self.preview_features.contains(&feature.to_string())
    }
}

impl PreviewFeatures for Option<&Generator> {
    fn has_preview_feature(&self, feature: &str) -> bool {
        match self {
            Some(gen) => gen.has_preview_feature(feature),
            None => false,
        }
    }
}
