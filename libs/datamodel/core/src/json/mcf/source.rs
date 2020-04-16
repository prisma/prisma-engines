use crate::{configuration, StringFromEnvVar};
use serde_json;

#[serde(rename_all = "camelCase")]
#[derive(Debug, serde::Serialize)]
pub struct SourceConfig {
    pub name: String,
    pub connector_type: String,
    pub url: StringFromEnvVar,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

pub fn render_sources_to_json_value(sources: &[Box<dyn configuration::Source + Send + Sync>]) -> serde_json::Value {
    let res = sources_to_json_structs(sources);
    serde_json::to_value(&res).expect("Failed to render JSON.")
}

pub fn render_sources_to_json(sources: &[Box<dyn configuration::Source + Send + Sync>]) -> String {
    let res = sources_to_json_structs(sources);
    serde_json::to_string_pretty(&res).expect("Failed to render JSON.")
}

fn sources_to_json_structs(sources: &[Box<dyn configuration::Source + Send + Sync>]) -> Vec<SourceConfig> {
    let mut res: Vec<SourceConfig> = Vec::new();

    for source in sources {
        res.push(source_to_json_struct(&**source));
    }

    res
}

fn source_to_json_struct(source: &dyn configuration::Source) -> SourceConfig {
    SourceConfig {
        name: source.name().clone(),
        connector_type: String::from(source.connector_type()),
        url: source.url().clone(),
        documentation: source.documentation().clone(),
    }
}
