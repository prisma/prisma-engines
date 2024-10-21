use std::sync::{Arc, OnceLock};

use metrics::{Counter, CounterFn, Gauge, GaugeFn, Histogram, HistogramFn, Key, Recorder, Unit};
use metrics::{KeyName, Metadata, SharedString};

use crate::common::{MetricAction, MetricType};
use crate::registry::MetricVisitor;
use crate::MetricRegistry;

static GLOBAL_RECORDER: OnceLock<Option<Arc<MetricRecorder>>> = const { OnceLock::new() };

fn set_global_recorder(recorder: Arc<MetricRecorder>) {
    _ = GLOBAL_RECORDER.set(Some(recorder));
}

pub(crate) fn global_recorder() -> Option<Arc<MetricRecorder>> {
    GLOBAL_RECORDER.get()?.clone()
}

pub struct MetricRecorder {
    registry: MetricRegistry,
}

impl MetricRecorder {
    pub fn new(registry: MetricRegistry) -> Self {
        Self { registry }
    }

    pub fn init_registry(&self) {
        metrics::with_local_recorder(self, || {
            super::initialize_metrics();
        });
    }

    pub fn install_globally(self: Arc<Self>) {
        set_global_recorder(self);
        metrics::set_global_recorder(self);
    }

    fn register_description(&self, name: KeyName, description: &str) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Description,
            action: MetricAction::Description(description.to_owned()),
            name: Key::from_name(name),
        });
        // trace!(
        //     target: METRIC_TARGET,
        //     name = name,
        //     metric_type = METRIC_DESCRIPTION,
        //     description = description
        // );
    }

    fn record_in_registry(&self, visitor: &MetricVisitor) {
        self.registry.record(visitor);
    }
}

impl Recorder for MetricRecorder {
    fn describe_counter(&self, key_name: KeyName, _unit: Option<Unit>, description: SharedString) {
        self.register_description(key_name, &description);
    }

    fn describe_gauge(&self, key_name: KeyName, _unit: Option<Unit>, description: SharedString) {
        self.register_description(key_name, &description);
    }

    fn describe_histogram(&self, key_name: KeyName, _unit: Option<Unit>, description: SharedString) {
        self.register_description(key_name, &description);
    }

    fn register_counter(&self, key: &Key, _metadata: &Metadata<'_>) -> Counter {
        Counter::from_arc(Arc::new(MetricHandle::new(key.clone(), self.registry.clone())))
    }

    fn register_gauge(&self, key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        Gauge::from_arc(Arc::new(MetricHandle::new(key.clone(), self.registry.clone())))
    }

    fn register_histogram(&self, key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        Histogram::from_arc(Arc::new(MetricHandle::new(key.clone(), self.registry.clone())))
    }
}

pub(crate) struct MetricHandle {
    key: Key,
    registry: MetricRegistry,
}

impl MetricHandle {
    pub fn new(key: Key, registry: MetricRegistry) -> Self {
        Self { key, registry }
    }

    fn record_in_registry(&self, visitor: &MetricVisitor) {
        self.registry.record(visitor);
    }
}

impl CounterFn for MetricHandle {
    fn increment(&self, value: u64) {
        // let keylabels: KeyLabels = self.key.clone().into();
        // let json_string = serde_json::to_string(&keylabels).unwrap();
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Counter,
            action: MetricAction::Increment(value),
            name: self.key.clone(),
        });
        // trace!(
        //     target: METRIC_TARGET,
        //     key_labels = json_string.as_str(),
        //     metric_type = METRIC_COUNTER,
        //     increment = value,
        // );
    }

    fn absolute(&self, value: u64) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Counter,
            action: MetricAction::Absolute(value),
            name: self.key.clone(),
        });
        // let keylabels: KeyLabels = self.key.clone().into();
        // let json_string = serde_json::to_string(&keylabels).unwrap();
        // trace!(
        //     target: METRIC_TARGET,
        //     key_labels = json_string.as_str(),
        //     metric_type = METRIC_COUNTER,
        //     absolute = value,
        // );
    }
}

impl GaugeFn for MetricHandle {
    fn increment(&self, value: f64) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Gauge,
            action: MetricAction::GaugeInc(value),
            name: self.key.clone(),
        });
        // let keylabels: KeyLabels = self.key.clone().into();
        // let json_string = serde_json::to_string(&keylabels).unwrap();
        // trace!(
        //     target: METRIC_TARGET,
        //     key_labels = json_string.as_str(),
        //     metric_type = METRIC_GAUGE,
        //     gauge_inc = value,
        // );
    }

    fn decrement(&self, value: f64) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Gauge,
            action: MetricAction::GaugeDec(value),
            name: self.key.clone(),
        });
        // let keylabels: KeyLabels = self.key.clone().into();
        // let json_string = serde_json::to_string(&keylabels).unwrap();
        // trace!(
        //     target: METRIC_TARGET,
        //     key_labels = json_string.as_str(),
        //     metric_type = METRIC_GAUGE,
        //     gauge_dec = value,
        // );
    }

    fn set(&self, value: f64) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Gauge,
            action: MetricAction::GaugeSet(value),
            name: self.key.clone(),
        });
        // let keylabels: KeyLabels = self.key.clone().into();
        // let json_string = serde_json::to_string(&keylabels).unwrap();
        // trace!(
        //     target: METRIC_TARGET,
        //     key_labels = json_string.as_str(),
        //     metric_type = METRIC_GAUGE,
        //     gauge_set = value,
        // );
    }
}

impl HistogramFn for MetricHandle {
    fn record(&self, value: f64) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Histogram,
            action: MetricAction::HistRecord(value),
            name: self.key.clone(),
        });
        // let keylabels: KeyLabels = self.key.clone().into();
        // let json_string = serde_json::to_string(&keylabels).unwrap();
        // trace!(
        //     target: METRIC_TARGET,
        //     key_labels = json_string.as_str(),
        //     metric_type = METRIC_HISTOGRAM,
        //     hist_record = value,
        // );
    }
}
