use crate::{configuration::StringFromEnvVar, PreviewFeature};
use enumflags2::BitFlags;
use serde::{ser::SerializeSeq, Serialize, Serializer};
use std::collections::HashMap;
use tsify::Tsify;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(Tsify)]
#[tsify(into_wasm_abi)]
pub struct Generator {
    pub name: String,
    pub provider: StringFromEnvVar,
    pub output: Option<StringFromEnvVar>,
    pub config: HashMap<String, String>,

    pub binary_targets: Vec<StringFromEnvVar>,

    #[serde(serialize_with = "mcf_preview_features")]
    #[tsify(type = "string[]")]
    pub preview_features: BitFlags<PreviewFeature>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,

    #[serde(default, skip)]
    pub is_custom_output: bool,
}

pub fn mcf_preview_features<S>(feats: &BitFlags<PreviewFeature>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = s.serialize_seq(Some(feats.len()))?;
    for feat in feats.iter() {
        seq.serialize_element(&feat)?;
    }
    seq.end()
}
