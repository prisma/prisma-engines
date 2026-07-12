use itertools::Itertools;
use parser_database::Files;
use serde::Serialize;

use crate::configuration::Generator;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtendedGenerator<'a> {
    #[serde(flatten)]
    pub generator: &'a Generator,
    pub source_file_path: &'a str,
}

pub fn generators_to_json_value(generators: &[Generator], files: &Files) -> serde_json::Value {
    serde_json::to_value(
        generators
            .iter()
            .map(|generator| ExtendedGenerator {
                generator,
                source_file_path: &files[generator.span.file_id].0,
            })
            .collect_vec(),
    )
    .expect("Failed to render JSON.")
}

pub fn generators_to_json(generators: &[Generator]) -> String {
    serde_json::to_string_pretty(generators).expect("Failed to render JSON.")
}
