use indexmap::IndexMap;
use metrics::{Counter, CounterFn, Gauge, GaugeFn, Histogram, HistogramFn, Key, Recorder, Unit};
use metrics::{KeyName, Label};
use metrics_util::{
    registry::{GenerationalAtomicStorage, GenerationalStorage, Registry},
    Histogram as HistogramUtil,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use tracing::{
    field::{Field, Visit},
    Subscriber,
};
use tracing_subscriber::Layer;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const METRIC_TARGET: &str = "qe_metrics";
const METRIC_COUNTER: &str = "counter";
const METRIC_GAUGE: &str = "gauge";
const METRIC_HISTOGRAM: &str = "histogram";
const METRIC_DESCRIPTION: &str = "description";

mod formatters;
use formatters::{metrics_to_json, Metric, MetricValue, Snapshot};

// At the moment the histogram is only used for timings. So the bounds are hard coded here
// The buckets are for ms
const HISTOGRAM_BOUNDS: [f64; 13] = [
    0.0, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0,
];
// We need a list of acceptable metrics we want to expose, we don't want to accidentally expose metrics
// that a different library have or unintended information
const ACCEPT_LIST: &'static [&'static str] = &[
    "pool.active_connections",
    "pool.idle_connections",
    "pool.wait_count",
    "pool.wait_duration",
];

struct Inner {
    descriptions: RwLock<HashMap<String, String>>,
    register: Registry<Key, GenerationalAtomicStorage>,
    accept_list: Vec<&'static str>,
}

impl Inner {
    fn new(accept_list: Vec<&'static str>) -> Self {
        Self {
            descriptions: RwLock::new(HashMap::new()),
            register: Registry::new(GenerationalStorage::atomic()),
            accept_list,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct KeyLabels {
    name: String,
    labels: IndexMap<String, String>,
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

#[derive(Clone)]
pub struct MetricRegistry {
    inner: Arc<Inner>,
}

impl fmt::Debug for MetricRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Metric Registry")
    }
}

impl MetricRegistry {
    pub fn new() -> Self {
        Self::new_with_accept_list(ACCEPT_LIST.to_vec())
    }

    // for internal and testing usage only
    fn new_with_accept_list(accept_list: Vec<&'static str>) -> Self {
        MetricRegistry {
            inner: Arc::new(Inner::new(accept_list)),
        }
    }

    fn record(&self, metric: &MetricVisitor) {
        match metric.metric_type {
            MetricType::Counter => self.handle_counter(metric),
            MetricType::Gauge => self.handle_gauge(metric),
            MetricType::Histogram => self.handle_histogram(metric),
            MetricType::Description => self.handle_description(metric),
        }
    }

    fn handle_description(&self, metric: &MetricVisitor) {
        if let MetricAction::Description(description) = &metric.action {
            let mut descriptions = self.inner.descriptions.write();
            //TODO: Sanitize string
            descriptions
                .entry(metric.name.name().to_string())
                .or_insert(description.to_string());
        }
    }

    fn handle_counter(&self, metric: &MetricVisitor) {
        self.inner
            .register
            .get_or_create_counter(&metric.name, |c| match metric.action {
                MetricAction::Increment(val) => CounterFn::increment(c, val),
                MetricAction::Absolute(val) => CounterFn::absolute(c, val),
                _ => (),
            });
    }

    fn handle_gauge(&self, metric: &MetricVisitor) {
        self.inner
            .register
            .get_or_create_gauge(&metric.name, |c| match metric.action {
                MetricAction::GaugeInc(val) => GaugeFn::increment(c, val as f64),
                MetricAction::GaugeSet(val) => GaugeFn::set(c, val as f64),
                MetricAction::GaugeDec(val) => GaugeFn::decrement(c, val as f64),
                _ => (),
            });
    }

    fn handle_histogram(&self, metric: &MetricVisitor) {
        self.inner
            .register
            .get_or_create_histogram(&metric.name, |c| match metric.action {
                MetricAction::HistRecord(val) => HistogramFn::record(c, val as f64),
                _ => (),
            });
    }

    pub fn counter_value(&self, name: &'static str) -> Option<u64> {
        let key = Key::from_name(name);
        let counters = self.inner.register.get_counter_handles();
        let counter = counters.get(&key)?;
        Some(counter.get_inner().load(Ordering::Acquire))
    }

    pub fn gauge_value(&self, name: &'static str) -> Option<f64> {
        let key = Key::from_name(name);
        let gauges = self.inner.register.get_gauge_handles();
        let gauge = gauges.get(&key)?;
        let val = f64::from_bits(gauge.get_inner().load(Ordering::Acquire));
        Some(val)
    }

    pub fn histogram_values(&self, name: &'static str) -> Option<HistogramUtil> {
        let mut histogram = HistogramUtil::new(&HISTOGRAM_BOUNDS)?;
        let key = Key::from_name(name);
        let histograms = self.inner.register.get_histogram_handles();
        let samples = histograms.get(&key)?;

        samples.get_inner().data_with(|s| {
            histogram.record_many(s);
        });
        Some(histogram)
    }

    pub fn get_descriptions(&self) -> HashMap<String, String> {
        let descriptions = self.inner.descriptions.read();
        descriptions.clone()
    }

    fn get_snapshot(&self) -> Snapshot {
        let counter_handles = self.inner.register.get_counter_handles();
        let gauge_handles = self.inner.register.get_gauge_handles();
        let histogram_handles = self.inner.register.get_histogram_handles();
        let descriptions = self.get_descriptions();

        let mut counters: Vec<Metric> = counter_handles
            .into_iter()
            .map(|(key, counter)| {
                let key_name = key.name();
                let value = counter.get_inner().load(Ordering::Acquire);
                let description = descriptions.get(key_name).cloned().unwrap_or_default();
                Metric::new(key, description, MetricValue::Counter(value))
            })
            .collect();

        let mut gauges: Vec<Metric> = gauge_handles
            .into_iter()
            .map(|(key, gauge)| {
                let key_name = key.name();
                let description = descriptions.get(key_name).cloned().unwrap_or_default();
                let value = f64::from_bits(gauge.get_inner().load(Ordering::Acquire));
                Metric::new(key, description, MetricValue::Gauge(value))
            })
            .collect();

        let mut histograms: Vec<Metric> = histogram_handles
            .into_iter()
            .map(|(key, samples)| {
                let mut histogram = HistogramUtil::new(&HISTOGRAM_BOUNDS).unwrap();
                samples.get_inner().data_with(|s| {
                    histogram.record_many(s);
                });

                let key_name = key.name();
                let description = descriptions.get(key_name).cloned().unwrap_or_default();
                let value = histogram.buckets();
                Metric::new(key, description, MetricValue::Histogram(value))
            })
            .collect();

        // Sort them so that they are in ordered by key name
        counters.sort_by(|a, b| a.key.cmp(&b.key));
        gauges.sort_by(|a, b| a.key.cmp(&b.key));
        histograms.sort_by(|a, b| a.key.cmp(&b.key));

        Snapshot {
            counters,
            gauges,
            histograms,
        }
    }

    pub fn to_json(&self) -> Value {
        let metrics = self.get_snapshot();
        metrics_to_json(metrics)
    }

    fn is_accepted_metric(&self, visitor: &MetricVisitor) -> bool {
        let name = visitor.name.name();
        if self.inner.accept_list.contains(&name) {
            return true;
        }

        false
    }
}

#[derive(Debug)]
enum MetricType {
    Counter,
    Gauge,
    Histogram, // Histograms are cumulative
    Description,
}

#[derive(Debug)]
enum MetricAction {
    Increment(u64),
    Absolute(u64),
    GaugeSet(f64),
    GaugeInc(f64),
    GaugeDec(f64),
    HistRecord(f64),
    Description(String),
}

#[derive(Debug)]
struct MetricVisitor {
    metric_type: MetricType,
    action: MetricAction,
    name: Key,
}

impl MetricVisitor {
    pub fn new() -> Self {
        Self {
            metric_type: MetricType::Description,
            action: MetricAction::Absolute(0),
            name: Key::from_name(""),
        }
    }
}

impl Visit for MetricVisitor {
    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}

    fn record_f64(&mut self, field: &Field, value: f64) {
        match field.name() {
            "gauge_inc" => self.action = MetricAction::GaugeInc(value),
            "gauge_dec" => self.action = MetricAction::GaugeDec(value),
            "gauge_set" => self.action = MetricAction::GaugeSet(value),
            "hist_record" => self.action = MetricAction::HistRecord(value),
            _ => (),
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        match field.name() {
            "increment" => self.action = MetricAction::Increment(value as u64),
            "absolute" => self.action = MetricAction::Absolute(value as u64),
            _ => (),
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        match field.name() {
            "increment" => self.action = MetricAction::Increment(value),
            "absolute" => self.action = MetricAction::Absolute(value),
            _ => (),
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match (field.name(), value) {
            ("metric_type", METRIC_COUNTER) => self.metric_type = MetricType::Counter,
            ("metric_type", METRIC_GAUGE) => self.metric_type = MetricType::Gauge,
            ("metric_type", METRIC_HISTOGRAM) => self.metric_type = MetricType::Histogram,
            ("metric_type", METRIC_DESCRIPTION) => self.metric_type = MetricType::Description,
            ("name", _) => self.name = Key::from_name(value.to_string()),
            ("key_labels", _) => {
                let key_labels: KeyLabels = serde_json::from_str(value).unwrap();
                self.name = key_labels.into();
            }
            (METRIC_DESCRIPTION, _) => self.action = MetricAction::Description(value.to_string()),
            _ => (),
        }
    }
}

// A tracing layer for receiving metric trace events and storing them in the registry.
impl<S: Subscriber> Layer<S> for MetricRegistry {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if event.metadata().target() != METRIC_TARGET {
            return;
        }
        println!("event {:?}", event);

        let mut visitor = MetricVisitor::new();
        event.record(&mut visitor);

        if self.is_accepted_metric(&visitor) {
            self.record(&visitor);
        }
    }
}

struct MetricHandle(Key);

impl CounterFn for MetricHandle {
    fn increment(&self, value: u64) {
        let keylabels: KeyLabels = self.0.clone().into();
        let json_string = serde_json::to_string(&keylabels).unwrap();
        trace!(
            target: METRIC_TARGET,
            key_labels = json_string.as_str(),
            metric_type = METRIC_COUNTER,
            increment = value,
        );
    }

    fn absolute(&self, value: u64) {
        let keylabels: KeyLabels = self.0.clone().into();
        let json_string = serde_json::to_string(&keylabels).unwrap();
        trace!(
            target: METRIC_TARGET,
            key_labels = json_string.as_str(),
            metric_type = METRIC_COUNTER,
            absolute = value,
        );
    }
}

impl GaugeFn for MetricHandle {
    fn increment(&self, value: f64) {
        let keylabels: KeyLabels = self.0.clone().into();
        let json_string = serde_json::to_string(&keylabels).unwrap();
        trace!(
            target: METRIC_TARGET,
            key_labels = json_string.as_str(),
            metric_type = METRIC_GAUGE,
            gauge_inc = value,
        );
    }

    fn decrement(&self, value: f64) {
        let keylabels: KeyLabels = self.0.clone().into();
        let json_string = serde_json::to_string(&keylabels).unwrap();
        trace!(
            target: METRIC_TARGET,
            key_labels = json_string.as_str(),
            metric_type = METRIC_GAUGE,
            gauge_dec = value,
        );
    }

    fn set(&self, value: f64) {
        let keylabels: KeyLabels = self.0.clone().into();
        let json_string = serde_json::to_string(&keylabels).unwrap();
        trace!(
            target: METRIC_TARGET,
            key_labels = json_string.as_str(),
            metric_type = METRIC_GAUGE,
            gauge_set = value,
        );
    }
}

impl HistogramFn for MetricHandle {
    fn record(&self, value: f64) {
        let keylabels: KeyLabels = self.0.clone().into();
        let json_string = serde_json::to_string(&keylabels).unwrap();
        trace!(
            target: METRIC_TARGET,
            key_labels = json_string.as_str(),
            metric_type = METRIC_HISTOGRAM,
            hist_record = value,
        );
    }
}

#[derive(Default)]
struct MetricRecorder;

impl MetricRecorder {
    fn register_description(&self, name: &str, description: &str) {
        trace!(
            target: METRIC_TARGET,
            name = name,
            metric_type = METRIC_DESCRIPTION,
            description = description
        );
    }
}

impl Recorder for MetricRecorder {
    fn describe_counter(&self, key_name: KeyName, _unit: Option<Unit>, description: &'static str) {
        self.register_description(key_name.as_str(), description);
    }

    fn describe_gauge(&self, key_name: KeyName, _unit: Option<Unit>, description: &'static str) {
        self.register_description(key_name.as_str(), description);
    }

    fn describe_histogram(&self, key_name: KeyName, _unit: Option<Unit>, description: &'static str) {
        self.register_description(key_name.as_str(), description);
    }

    fn register_counter(&self, key: &Key) -> Counter {
        Counter::from_arc(Arc::new(MetricHandle(key.clone())))
    }

    fn register_gauge(&self, key: &Key) -> Gauge {
        Gauge::from_arc(Arc::new(MetricHandle(key.clone())))
    }

    fn register_histogram(&self, key: &Key) -> Histogram {
        Histogram::from_arc(Arc::new(MetricHandle(key.clone())))
    }
}

pub fn set_recorder() {
    let recorder = MetricRecorder::default();
    metrics::set_boxed_recorder(Box::new(recorder)).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use metrics::{
        absolute_counter, decrement_gauge, describe_counter, describe_gauge, describe_histogram, gauge, histogram,
        increment_counter, increment_gauge, register_counter, register_gauge, register_histogram,
    };
    use serde_json::json;
    use tracing::instrument::WithSubscriber;
    use tracing::Dispatch;
    use tracing_subscriber::layer::SubscriberExt;

    use once_cell::sync::Lazy;
    use tokio::runtime::Runtime;

    static RT: Lazy<Runtime> = Lazy::new(|| {
        set_recorder();
        Runtime::new().unwrap()
    });

    const TESTING_ACCEPT_LIST: &'static [&'static str] = &[
        "test_counter",
        "another_counter",
        "test_gauge",
        "another_gauge",
        "test_histogram",
        "counter_1",
        "counter_2",
        "gauge_1",
        "gauge_2",
        "histogram_1",
        "histogram_2",
    ];

    #[test]
    fn test_counters() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let counter1 = register_counter!("test_counter");
                counter1.increment(1);
                increment_counter!("test_counter");
                increment_counter!("test_counter");

                increment_counter!("another_counter");

                let val = metrics.counter_value("test_counter").unwrap();
                assert_eq!(val, 3);

                let val2 = metrics.counter_value("another_counter").unwrap();
                assert_eq!(val2, 1);

                absolute_counter!("test_counter", 5);
                let val3 = metrics.counter_value("test_counter").unwrap();
                assert_eq!(val3, 5);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_gauges() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let gauge1 = register_gauge!("test_gauge");
                gauge1.increment(1.0);
                increment_gauge!("test_gauge", 1.0);
                increment_gauge!("test_gauge", 1.0);
                increment_gauge!("another_gauge", 1.0);

                let val = metrics.gauge_value("test_gauge").unwrap();
                assert_eq!(val, 3.0);

                let val2 = metrics.gauge_value("another_gauge").unwrap();
                assert_eq!(val2, 1.0);

                assert_eq!(None, metrics.counter_value("test_gauge"));

                gauge!("test_gauge", 5.0);
                let val3 = metrics.gauge_value("test_gauge").unwrap();
                assert_eq!(val3, 5.0);

                decrement_gauge!("test_gauge", 2.0);
                let val4 = metrics.gauge_value("test_gauge").unwrap();
                assert_eq!(val4, 3.0);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_no_panic_and_ignore_other_traces() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                trace!("a fake trace");

                increment_gauge!("test_gauge", 1.0);
                increment_counter!("test_counter");

                trace!("another fake trace");

                assert_eq!(1.0, metrics.gauge_value("test_gauge").unwrap());
                assert_eq!(1, metrics.counter_value("test_counter").unwrap());
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_ignore_non_accepted_metrics() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                increment_gauge!("not_accepted", 1.0);
                increment_gauge!("test_gauge", 1.0);

                assert_eq!(1.0, metrics.gauge_value("test_gauge").unwrap());
                assert_eq!(None, metrics.gauge_value("not_accepted"));
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_histograms() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let hist = register_histogram!("test_histogram");
                hist.record(9.0);

                histogram!("test_histogram", 100.0);
                histogram!("test_histogram", 0.1);

                histogram!("test_histogram", 1999.0);
                histogram!("test_histogram", 3999.0);
                histogram!("test_histogram", 610.0);

                let hist = metrics.histogram_values("test_histogram").unwrap();
                let expected: Vec<(f64, u64)> = Vec::from([
                    (10.0, 2),
                    (20.0, 2),
                    (50.0, 2),
                    (100.0, 3),
                    (200.0, 3),
                    (500.0, 3),
                    (1000.0, 4),
                    (2000.0, 5),
                    (5000.0, 6),
                ]);

                assert_eq!(hist.buckets(), expected);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_labels() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let hist = register_histogram!("test_histogram", "label" => "one", "two" => "another");
                hist.record(9.0);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_set_and_read_descriptions() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                describe_counter!("test_counter", "This is a counter");

                let descriptions = metrics.get_descriptions();
                let description = descriptions.get("test_counter").unwrap();

                assert_eq!("This is a counter", description);

                describe_gauge!("test_gauge", "This is a gauge");

                let descriptions = metrics.get_descriptions();
                let description = descriptions.get("test_gauge").unwrap();

                assert_eq!("This is a gauge", description);

                describe_histogram!("test_histogram", "This is a hist");

                let descriptions = metrics.get_descriptions();
                let description = descriptions.get("test_histogram").unwrap();
                assert_eq!("This is a hist", description);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }

    #[test]
    fn test_to_json() {
        RT.block_on(async {
            let metrics = MetricRegistry::new_with_accept_list(TESTING_ACCEPT_LIST.to_vec());
            let dispatch = Dispatch::new(tracing_subscriber::Registry::default().with(metrics.clone()));
            async {
                let empty = json!({
                    "counters": [],
                    "gauges": [],
                    "histograms": []
                });

                assert_eq!(metrics.to_json(), empty);

                absolute_counter!("counter_1", 4, "label" => "one");
                let _ = describe_counter!("counter_2", "this is a description for counter 2");
                absolute_counter!("counter_2", 2, "label" => "one", "another_label" => "two");

                describe_gauge!("gauge_1", "a description for gauge 1");
                gauge!("gauge_1", 7.0);
                gauge!("gauge_2", 3.0, "label" => "three");

                describe_histogram!("histogram_1", "a description for histogram");
                let hist = register_histogram!("histogram_1", "label" => "one", "hist_two" => "two");
                hist.record(9.0);

                histogram!("histogram_2", 1000.0);

                let json = metrics.to_json();
                let expected = json!({
                    "counters": [{
                        "key": "counter_1",
                        "value": 4,
                        "labels": {"label": "one"},
                        "description": ""
                    },{
                        "key": "counter_2",
                        "value": 2,
                        "labels": {"label": "one", "another_label": "two"},
                        "description": "this is a description for counter 2"
                    }],
                    "gauges": [{
                        "key": "gauge_1",
                        "value": 7.0,
                        "labels": {},
                        "description": "a description for gauge 1"
                    }, {
                        "key": "gauge_2",
                        "value": 3.0,
                        "labels": {"label": "three"},
                        "description": ""
                    }],
                    "histograms": [{
                        "key": "histogram_1",
                        "value":[[10.0,1],[20.0,1],[50.0,1],[100.0,1],[200.0,1],[500.0,1],[1000.0,1],[2000.0,1],[5000.0,1]],
                        "labels": {"label": "one", "hist_two": "two"},
                        "description": "a description for histogram"
                    }, {
                        "key": "histogram_2",
                        "value":[[10.0,0],[20.0,0],[50.0,0],[100.0,0],[200.0,0],[500.0,0],[1000.0,1],[2000.0,1],[5000.0,1]],
                        "labels": {},
                        "description": ""
                    }]
                });

                assert_eq!(expected, json);
            }
            .with_subscriber(dispatch)
            .await;
        });
    }
}
