use std::collections::HashMap;
use std::fmt;
use std::sync::{atomic::Ordering, Arc};

use metrics::{CounterFn, GaugeFn, HistogramFn, Key};
use metrics_util::{
    registry::{GenerationalAtomicStorage, GenerationalStorage, Registry},
    Histogram as HistogramUtil,
};
use parking_lot::RwLock;
use serde_json::Value;

use super::formatters::metrics_to_json;
use super::{
    common::{Metric, MetricAction, MetricType, MetricValue, Snapshot},
    formatters::metrics_to_prometheus,
};
use super::{ACCEPT_LIST, HISTOGRAM_BOUNDS};

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

#[derive(Clone)]
pub struct MetricRegistry {
    inner: Arc<Inner>,
}

impl fmt::Debug for MetricRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MetricRegistry {{ .. }}")
    }
}

impl Default for MetricRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricRegistry {
    pub fn new() -> Self {
        Self::new_with_accept_list(ACCEPT_LIST.to_vec())
    }

    // for internal and testing usage only
    pub(crate) fn new_with_accept_list(accept_list: Vec<&'static str>) -> Self {
        MetricRegistry {
            inner: Arc::new(Inner::new(accept_list)),
        }
    }

    pub(crate) fn record(&self, metric: &MetricVisitor) {
        if self.is_accepted_metric(metric) {
            match metric.metric_type {
                MetricType::Counter => self.handle_counter(metric),
                MetricType::Gauge => self.handle_gauge(metric),
                MetricType::Histogram => self.handle_histogram(metric),
                MetricType::Description => self.handle_description(metric),
            }
        }
    }

    fn handle_description(&self, metric: &MetricVisitor) {
        if let MetricAction::Description(description) = &metric.action {
            let mut descriptions = self.inner.descriptions.write();
            //TODO: Sanitize string
            descriptions
                .entry(metric.name.name().to_string())
                .or_insert_with(|| description.to_string());
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
                MetricAction::GaugeInc(val) => GaugeFn::increment(c, val),
                MetricAction::GaugeSet(val) => GaugeFn::set(c, val),
                MetricAction::GaugeDec(val) => GaugeFn::decrement(c, val),
                _ => (),
            });
    }

    fn handle_histogram(&self, metric: &MetricVisitor) {
        self.inner.register.get_or_create_histogram(&metric.name, |c| {
            if let MetricAction::HistRecord(val) = metric.action {
                // We multiply by 1000 here because the value is converted into seconds when doing:
                // histogram!("my_histogram", duration.elapsed());
                // and we want it in ms
                HistogramFn::record(c, val * 1000.0)
            }
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

    fn get_snapshot(&self, global_labels: HashMap<String, String>) -> Snapshot {
        let counter_handles = self.inner.register.get_counter_handles();
        let gauge_handles = self.inner.register.get_gauge_handles();
        let histogram_handles = self.inner.register.get_histogram_handles();
        let descriptions = self.get_descriptions();

        let mut counters: Vec<Metric> = counter_handles
            .into_iter()
            .map(|(key, counter)| {
                let value = counter.get_inner().load(Ordering::Acquire);
                Metric::renamed(key, &descriptions, MetricValue::Counter(value), &global_labels)
            })
            .collect();

        let mut gauges: Vec<Metric> = gauge_handles
            .into_iter()
            .map(|(key, gauge)| {
                let value = f64::from_bits(gauge.get_inner().load(Ordering::Acquire));
                Metric::renamed(key, &descriptions, MetricValue::Gauge(value), &global_labels)
            })
            .collect();

        let mut histograms: Vec<Metric> = histogram_handles
            .into_iter()
            .map(|(key, samples)| {
                let mut histogram = HistogramUtil::new(&HISTOGRAM_BOUNDS).unwrap();
                samples.get_inner().data_with(|s| {
                    histogram.record_many(s);
                });

                Metric::renamed(
                    key,
                    &descriptions,
                    MetricValue::Histogram(histogram.into()),
                    &global_labels,
                )
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

    pub fn to_json(&self, global_labels: HashMap<String, String>) -> Value {
        let metrics = self.get_snapshot(global_labels);
        metrics_to_json(metrics)
    }

    pub fn to_prometheus(&self, global_labels: HashMap<String, String>) -> String {
        let metrics = self.get_snapshot(global_labels);
        metrics_to_prometheus(metrics)
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
pub(crate) struct MetricVisitor {
    pub(crate) metric_type: MetricType,
    pub(crate) action: MetricAction,
    pub(crate) name: Key,
}
