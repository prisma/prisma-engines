use crate::configuration::Encryptor;

pub fn encryptors_to_json_value(encryptors: &[Encryptor]) -> serde_json::Value {
    serde_json::to_value(encryptors).expect("Failed to render JSON.")
}

pub fn encryptors_to_json(encryptors: &[Encryptor]) -> String {
    serde_json::to_string_pretty(encryptors).expect("Failed to render JSON.")
}
