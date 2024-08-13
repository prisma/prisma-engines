use parser_database::Files;
use schema_ast::ast::WithSpan;

use crate::configuration::{self, StringFromEnvVar};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceConfig {
    pub name: String,
    pub provider: String,
    pub active_provider: String,
    pub url: StringFromEnvVar,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_url: Option<StringFromEnvVar>,
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    pub source_file_path: String,
}

pub fn render_sources_to_json_value(sources: &[configuration::Datasource], files: &Files) -> serde_json::Value {
    let res = sources_to_json_structs(sources, files);
    serde_json::to_value(res).expect("Failed to render JSON.")
}

pub fn render_sources_to_json(sources: &[configuration::Datasource], files: &Files) -> String {
    let res = sources_to_json_structs(sources, files);
    serde_json::to_string_pretty(&res).expect("Failed to render JSON.")
}

fn sources_to_json_structs(sources: &[configuration::Datasource], files: &Files) -> Vec<SourceConfig> {
    let mut res: Vec<SourceConfig> = Vec::new();

    for source in sources {
        res.push(source_to_json_struct(source, files));
    }

    res
}

fn source_to_json_struct(source: &configuration::Datasource, files: &Files) -> SourceConfig {
    let schemas: Vec<String> = source
        .namespaces
        .iter()
        .map(|(namespace, _)| namespace.clone())
        .collect();

    SourceConfig {
        name: source.name.clone(),
        provider: source.provider.clone(),
        active_provider: source.active_provider.to_string(),
        url: source.url.clone(),
        direct_url: source.direct_url.clone(),
        documentation: source.documentation.clone(),
        schemas,
        source_file_path: files[source.span().file_id].0.clone(),
    }
}
