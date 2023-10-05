use std::collections::HashMap;

use metrics::{Key, Label};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct KeyLabels {
    name: String,
    labels: HashMap<String, String>,
}

#[derive(Debug)]
pub(crate) enum MetricType {
    Counter,
    Gauge,
    Histogram, // Histograms are cumulative
    Description,
}

#[derive(Debug)]
pub(crate) enum MetricAction {
    Increment(u64),
    Absolute(u64),
    GaugeSet(f64),
    GaugeInc(f64),
    GaugeDec(f64),
    HistRecord(f64),
    Description(String),
}

#[derive(Serialize, Clone)]
pub(crate) struct Histogram {
    pub buckets: Vec<(f64, u64)>,
    pub sum: f64,
    pub count: u64,
}

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub(crate) enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Histogram),
}

#[derive(Serialize, Clone)]
pub(crate) struct Metric {
    pub key: String,
    pub labels: HashMap<String, String>,
    pub value: MetricValue,
    pub description: String,
}

impl Metric {
    pub(crate) fn renamed(
        key: Key,
        descriptions: &HashMap<String, String>,
        value: MetricValue,
        global_labels: &HashMap<String, String>,
    ) -> Self {
        match crate::METRIC_RENAMES.get(key.name()) {
            Some((new_key, new_description)) => Self::new(
                Key::from_parts(new_key.to_string(), key.labels()),
                new_description.to_string(),
                value,
                global_labels.clone(),
            ),
            None => {
                let description = descriptions.get(key.name()).map(|s| s.to_string()).unwrap_or_default();
                Self::new(key, description, value, global_labels.clone())
            }
        }
    }

    fn new(key: Key, description: String, value: MetricValue, global_labels: HashMap<String, String>) -> Self {
        let (name, labels) = key.into_parts();

        let mut labels_map: HashMap<String, String> = labels
            .into_iter()
            .map(|label| (label.key().to_string(), label.value().to_string()))
            .collect();

        labels_map.extend(global_labels);

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
pub(crate) struct Snapshot {
    pub counters: Vec<Metric>,
    pub gauges: Vec<Metric>,
    pub histograms: Vec<Metric>,
}

impl From<Key> for KeyLabels {
    fn from(key: Key) -> Self {
        let mut kl = KeyLabels {
            name: key.name().to_string(),
            labels: Default::default(),
        };

        kl.labels
            .extend(key.labels().map(|l| (l.key().to_string(), l.value().to_string())));

        kl
    }
}

impl From<KeyLabels> for Key {
    fn from(kl: KeyLabels) -> Self {
        let labels: Vec<Label> = kl
            .labels
            .into_iter()
            .map(|(key, value)| Label::from(&(key, value)))
            .collect();

        Key::from_parts(kl.name, labels)
    }
}

impl From<metrics_util::Histogram> for Histogram {
    fn from(histogram: metrics_util::Histogram) -> Histogram {
        Histogram {
            buckets: histogram.buckets(),
            sum: histogram.sum(),
            count: histogram.count(),
        }
    }
}
