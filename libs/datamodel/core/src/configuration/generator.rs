use crate::{common::preview_features::PreviewFeature, configuration::StringFromEnvVar};
use enumflags2::BitFlags;
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
    pub binary_targets: Vec<StringFromEnvVar>,

    #[serde(default)]
    pub preview_features: Vec<PreviewFeature>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

impl Generator {
    pub fn preview_features(&self) -> BitFlags<PreviewFeature> {
        self.preview_features.iter().copied().collect()
    }
}
