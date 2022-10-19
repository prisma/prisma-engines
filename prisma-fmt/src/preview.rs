use psl::ALL_PREVIEW_FEATURES;

pub fn run() -> String {
    serde_json::to_string(&ALL_PREVIEW_FEATURES.active_features().iter().collect::<Vec<_>>()).unwrap()
}
