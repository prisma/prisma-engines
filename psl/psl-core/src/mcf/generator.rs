use crate::configuration::Generator;

pub fn generators_to_json_value(generators: &[Generator]) -> serde_json::Value {
    serde_json::to_value(generators).expect("Failed to render JSON.")
}

pub fn generators_to_json(generators: &[Generator]) -> String {
    serde_json::to_string_pretty(generators).expect("Failed to render JSON.")
}
