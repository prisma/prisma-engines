use datamodel::common::preview_features::GENERATOR;

pub fn run() -> String {
    serde_json::to_string(&GENERATOR.active_features()).unwrap()
}
