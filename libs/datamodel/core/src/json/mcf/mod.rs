mod generator;
mod source;

pub use generator::*;
pub use source::*;

use crate::Configuration;
use serde::Serialize;

pub fn config_to_mcf_json_value(mcf: &Configuration) -> serde_json::Value {
    serde_json::to_value(&model_to_serializable(&mcf)).expect("Failed to render JSON.")
}

pub fn config_to_mcf_json(mcf: &Configuration) -> String {
    serde_json::to_string(&model_to_serializable(&mcf)).expect("Failed to render JSON.")
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Serialize)]
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
