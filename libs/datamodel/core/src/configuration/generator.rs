use crate::{common::preview_features::PreviewFeature, configuration::StringFromEnvVar};
use enumflags2::BitFlags;
use serde::{Serialize, Serializer};
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

    #[serde(default, serialize_with = "mcf_preview_features")]
    pub preview_features: Option<Vec<PreviewFeature>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

pub fn mcf_preview_features<S>(feat: &Option<Vec<PreviewFeature>>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match feat {
        Some(feats) => feats.serialize(s),
        None => Vec::<String>::new().serialize(s),
    }
}

impl Generator {
    pub fn preview_features(&self) -> BitFlags<PreviewFeature> {
        match self.preview_features {
            Some(ref preview_features) => preview_features.iter().copied().collect(),
            None => BitFlags::empty(),
        }
    }
}
