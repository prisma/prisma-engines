mod generator;
mod source;

pub use generator::*;
pub use source::*;

use crate::diagnostics::ValidatedConfiguration;
use serde::Serialize;

pub fn config_to_mcf_json_value(mcf: &ValidatedConfiguration) -> serde_json::Value {
    serde_json::to_value(&model_to_serializable(&mcf)).expect("Failed to render JSON.")
}

#[serde(rename_all = "camelCase")]
#[derive(Debug, Serialize)]
pub struct SerializeableMcf {
    generators: serde_json::Value,
    datasources: serde_json::Value,
    warnings: Vec<String>,
}

fn model_to_serializable(config: &ValidatedConfiguration) -> SerializeableMcf {
    SerializeableMcf {
        generators: generator::generators_to_json_value(&config.subject.generators),
        datasources: source::render_sources_to_json_value(&config.subject.datasources),
        warnings: config.warnings.iter().map(|f| f.description()).collect(),
    }
}
