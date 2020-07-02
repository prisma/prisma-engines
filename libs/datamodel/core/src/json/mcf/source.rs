use crate::{configuration, StringFromEnvVar};
use serde_json;

#[serde(rename_all = "camelCase")]
#[derive(Debug, serde::Serialize)]
pub struct SourceConfig {
    pub name: String,
    pub provider: Vec<String>,
    pub active_provider: String,
    pub url: StringFromEnvVar,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

pub fn render_sources_to_json_value(sources: &[configuration::Datasource]) -> serde_json::Value {
    let res = sources_to_json_structs(sources);
    serde_json::to_value(&res).expect("Failed to render JSON.")
}

pub fn render_sources_to_json(sources: &[configuration::Datasource]) -> String {
    let res = sources_to_json_structs(sources);
    serde_json::to_string_pretty(&res).expect("Failed to render JSON.")
}

fn sources_to_json_structs(sources: &[configuration::Datasource]) -> Vec<SourceConfig> {
    let mut res: Vec<SourceConfig> = Vec::new();

    for source in sources {
        res.push(source_to_json_struct(&source));
    }

    res
}

fn source_to_json_struct(source: &configuration::Datasource) -> SourceConfig {
    SourceConfig {
        name: source.name.clone(),
        provider: source.provider.clone(),
        active_provider: source.active_provider.to_string(),
        url: source.url().clone(),
        documentation: source.documentation.clone(),
    }
}
