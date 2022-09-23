mod generator;
mod source;

pub use generator::*;
pub use source::*;

use serde::Serialize;

pub fn config_to_mcf_json_value(mcf: &crate::Configuration) -> serde_json::Value {
    serde_json::to_value(&model_to_serializable(mcf)).expect("Failed to render JSON.")
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializeableMcf {
    generators: serde_json::Value,
    datasources: serde_json::Value,
    warnings: Vec<String>,
}

fn model_to_serializable(config: &crate::Configuration) -> SerializeableMcf {
    SerializeableMcf {
        generators: generator::generators_to_json_value(&config.generators),
        datasources: source::render_sources_to_json_value(&config.datasources),
        warnings: config.warnings.iter().map(|f| f.message().to_owned()).collect(),
    }
}
