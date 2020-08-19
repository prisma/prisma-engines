use crate::configuration::preview_features::PreviewFeatures;
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

impl PreviewFeatures for Generator {
    fn preview_features(&self) -> &Vec<String> {
        &self.preview_features
    }
}

impl PreviewFeatures for Option<&Generator> {
    fn preview_features(&self) -> &Vec<String> {
        match self {
            Some(dat) => &dat.preview_features,
            _ => panic!(""),
        }
    }
}
