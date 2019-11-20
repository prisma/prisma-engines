mod generator;
mod source;

pub use generator::*;
pub use source::*;

use crate::{Configuration, SourceDefinition};
use serde::{Deserialize, Serialize};

pub fn config_to_mcf_json_value(mcf: &Configuration) -> serde_json::Value {
    serde_json::to_value(&model_to_serializable(&mcf)).expect("Failed to render JSON.")
}

pub fn config_to_mcf_json(mcf: &Configuration) -> String {
    serde_json::to_string(&model_to_serializable(&mcf)).expect("Failed to render JSON.")
}

pub fn config_from_mcf_json(json: &str) -> Configuration {
    let mcf: SerializeableMcf = serde_json::from_str(json).expect("Failed to parse JSON.");

    Configuration {
        generators: generator::generators_from_json_value(mcf.generators),
        datasources: source::sources_from_json_value_with_plugins(mcf.datasources, vec![]),
    }
}

pub fn config_from_mcf_json_value(json: serde_json::Value) -> Configuration {
    let mcf: SerializeableMcf = serde_json::from_value(json).expect("Failed to parse JSON.");

    Configuration::from(mcf)
}

#[allow(unused)]
fn config_from_mcf_json_value_with_plugins(
    json: serde_json::Value,
    plugins: Vec<Box<dyn SourceDefinition>>,
) -> Configuration {
    let mcf: SerializeableMcf = serde_json::from_value(json).expect("Failed to parse JSON.");

    Configuration {
        generators: generator::generators_from_json_value(mcf.generators),
        datasources: source::sources_from_json_value_with_plugins(mcf.datasources, plugins),
    }
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Serialize, Deserialize)]
pub struct SerializeableMcf {
    generators: serde_json::Value,
    datasources: serde_json::Value,
}

fn model_to_serializable(config: &Configuration) -> SerializeableMcf {
    SerializeableMcf {
        generators: generator::generators_to_json_value(&config.generators),
        datasources: source::render_sources_to_json_value(&config.datasources),
    }
}

impl From<SerializeableMcf> for Configuration {
    fn from(mcf: SerializeableMcf) -> Self {
        Self {
            generators: generator::generators_from_json_value(mcf.generators),
            datasources: source::sources_from_json_value_with_plugins(mcf.datasources, vec![]),
        }
    }
}
