use indexmap::IndexMap;
use metrics::Key;
use metrics_util::Histogram as HistogramUtil;
use serde::Serialize;
use serde_json::value::Value;
use std::collections::HashMap;

#[derive(Serialize)]
#[serde(untagged)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<(f64, u64)>),
}

#[derive(Serialize)]
pub struct Metric {
    pub key: String,
    labels: IndexMap<String, String>,
    value: MetricValue,
    description: String,
}

impl Metric {
    pub fn new(key: Key, description: String, value: MetricValue) -> Self {
        let (name, labels) = key.into_parts();
        let labels_map = labels.into_iter().fold(IndexMap::new(), |mut map, label| {
            map.insert(label.key().to_string(), label.value().to_string());
            map
        });

        Self {
            key: name.as_str().to_string(),
            value,
            description,
            labels: labels_map,
        }
    }
}

// The idea of this snapshot is take from
// https://github.com/metrics-rs/metrics/blob/558a3f93a4bb3958379ae6227c613a222aa813b5/metrics-exporter-prometheus/src/common.rs#L79
#[derive(Serialize)]
pub struct Snapshot {
    pub counters: Vec<Metric>,
    pub gauges: Vec<Metric>,
    pub histograms: Vec<Metric>,
}

pub fn metrics_to_json(snapshot: Snapshot) -> Value {
    serde_json::to_value(snapshot).unwrap()
}
