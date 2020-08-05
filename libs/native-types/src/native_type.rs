pub trait NativeType {
    fn to_json(&self) -> serde_json::Value;
}
