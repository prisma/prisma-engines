mod generator;
mod source;

pub use generator::*;
pub use source::*;

use serde::Serialize;

pub struct Configuration {
    pub generators: Vec<Generator>,
    pub datasources: Vec<Datasource>,
}

#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Serialize)]
pub struct StringFromEnvVar {
    pub from_env_var: Option<String>,
    pub value: String,
}

impl StringFromEnvVar {
    pub fn plain_value(value: String) -> StringFromEnvVar {
        StringFromEnvVar {
            from_env_var: None,
            value,
        }
    }
}
