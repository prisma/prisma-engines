mod generator;
mod source;

pub use generator::*;
pub use source::*;

use serde::{Deserialize, Serialize};

pub struct Configuration {
    pub generators: Vec<Generator>,
    pub datasources: Vec<Box<dyn Source>>,
}

#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StringFromEnvVar {
    pub from_env_var: Option<String>,
    pub value: String,
}
