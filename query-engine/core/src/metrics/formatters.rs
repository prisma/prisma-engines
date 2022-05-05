use super::common::Snapshot;
use serde_json::Value;

pub(crate) fn metrics_to_json(snapshot: Snapshot) -> Value {
    serde_json::to_value(snapshot).unwrap()
}
