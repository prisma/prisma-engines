use std::sync::Arc;

use metrics::{KeyName, Metadata, SharedString};
use metrics::{Counter, CounterFn, Gauge, GaugeFn, Histogram, HistogramFn, Key, Recorder, Unit};
use tracing::trace;

use super::common::KeyLabels;
use super::{METRIC_COUNTER, METRIC_DESCRIPTION, METRIC_GAUGE, METRIC_HISTOGRAM, METRIC_TARGET};

#[derive(Default)]
pub(crate) struct MetricRecorder;

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
    fn describe_counter(&self, key_name: KeyName, _unit: Option<Unit>, description: SharedString) {
        self.register_description(key_name.as_str(), &description);
    }

    fn describe_gauge(&self, key_name: KeyName, _unit: Option<Unit>, description: SharedString) {
        self.register_description(key_name.as_str(), &description);
    }

    fn describe_histogram(&self, key_name: KeyName, _unit: Option<Unit>, description: SharedString) {
        self.register_description(key_name.as_str(), &description);
    }

    fn register_counter(&self, key: &Key, _metadata: &Metadata<'_>) -> Counter {
        Counter::from_arc(Arc::new(MetricHandle(key.clone())))
    }

    fn register_gauge(&self, key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        Gauge::from_arc(Arc::new(MetricHandle(key.clone())))
    }

    fn register_histogram(&self, key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        Histogram::from_arc(Arc::new(MetricHandle(key.clone())))
    }
}

pub(crate) struct MetricHandle(Key);

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
