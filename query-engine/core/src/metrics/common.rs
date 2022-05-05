use indexmap::IndexMap;
use metrics::{Key, Label};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct KeyLabels {
    name: String,
    labels: IndexMap<String, String>,
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

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<(f64, u64)>),
}

#[derive(Serialize)]
pub(crate) struct Metric {
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
pub(crate) struct Snapshot {
    pub counters: Vec<Metric>,
    pub gauges: Vec<Metric>,
    pub histograms: Vec<Metric>,
}

impl From<Key> for KeyLabels {
    fn from(key: Key) -> Self {
        let mut kl = KeyLabels {
            name: key.name().to_string(),
            labels: IndexMap::new(),
        };

        key.labels().for_each(|label| {
            kl.labels.insert(label.key().to_string(), label.value().to_string());
        });

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
