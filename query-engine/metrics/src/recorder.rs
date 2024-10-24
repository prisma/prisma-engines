use std::sync::{Arc, OnceLock};

use derive_more::Display;
use metrics::{Counter, CounterFn, Gauge, GaugeFn, Histogram, HistogramFn, Key, Recorder, Unit};
use metrics::{KeyName, Metadata, SharedString};

use crate::common::{MetricAction, MetricType};
use crate::registry::MetricVisitor;
use crate::MetricRegistry;

/// Default global metric recorder.
///
/// `metrics` crate has the state on its own. It allows setting the global recorder, it allows
/// overriding it for a duration of an async closure, and it allows borrowing the current recorder
/// for a short while. We, however, can't use this in our async instrumentation because we need the
/// current recorder to be `Send + 'static` to be able to store it in a future. The solution to
/// this is to maintain our own state in parallel. The APIs exposed by the crate guarantee that the
/// state is in sync.
///
/// Using `metrics::set_global_recorder` or `metrics::with_local_recorder` in user code is safe and
/// won't lead to any issues (even if the new recorder isn't the [`MetricRecorder`] from this
/// crate), however we won't know about any new local recorders on the stack, and calling
/// [`crate::WithMetricsInstrumentation::with_current_recorder`] will re-use the last
/// [`MetricRecorder`] known to us.
static GLOBAL_RECORDER: OnceLock<Option<MetricRecorder>> = const { OnceLock::new() };

#[derive(Display, Debug)]
#[display(fmt = "global recorder can only be installed once")]
pub struct AlreadyInstalled;

impl std::error::Error for AlreadyInstalled {}

fn set_global_recorder(recorder: MetricRecorder) -> Result<(), AlreadyInstalled> {
    GLOBAL_RECORDER.set(Some(recorder)).map_err(|_| AlreadyInstalled)
}

pub(crate) fn global_recorder() -> Option<MetricRecorder> {
    GLOBAL_RECORDER.get()?.clone()
}

/// Receives the metrics from the macros provided by the `metrics` crate and forwards them to
/// [`MetricRegistry`].
///
/// To provide an analogy, `MetricRecorder` to `MetricRegistry` is what `Dispatch` is to
/// `Subscriber` in `tracing`. Just like `Dispatch`, it acts like a handle to the registry and is
/// cheaply clonable with reference-counting semantics.
#[derive(Clone)]
pub struct MetricRecorder {
    registry: MetricRegistry,
}

impl MetricRecorder {
    pub fn new(registry: MetricRegistry) -> Self {
        Self { registry }
    }

    /// Convenience method to call [`Self::init_prisma_metrics`] immediately after creating the
    /// recorder.
    pub fn with_initialized_prisma_metrics(self) -> Self {
        self.init_prisma_metrics();
        self
    }

    /// Initializes the default Prisma metrics by dispatching their descriptions and initial values
    /// to the registry.
    ///
    /// Query engine needs this, but the metrics can also be used without this, especially in
    /// tests.
    pub fn init_prisma_metrics(&self) {
        metrics::with_local_recorder(self, || {
            super::initialize_metrics();
        });
    }

    /// Installs the metrics recorder globally, registering it both with the `metrics` crate and
    /// our own instrumentation.
    pub fn install_globally(&self) -> Result<(), AlreadyInstalled> {
        set_global_recorder(self.clone())?;
        metrics::set_global_recorder(self.clone()).map_err(|_| AlreadyInstalled)
    }

    fn register_description(&self, name: KeyName, description: &str) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Description,
            action: MetricAction::Description(description.to_owned()),
            name: Key::from_name(name),
        });
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
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Counter,
            action: MetricAction::Increment(value),
            name: self.key.clone(),
        });
    }

    fn absolute(&self, value: u64) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Counter,
            action: MetricAction::Absolute(value),
            name: self.key.clone(),
        });
    }
}

impl GaugeFn for MetricHandle {
    fn increment(&self, value: f64) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Gauge,
            action: MetricAction::GaugeInc(value),
            name: self.key.clone(),
        });
    }

    fn decrement(&self, value: f64) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Gauge,
            action: MetricAction::GaugeDec(value),
            name: self.key.clone(),
        });
    }

    fn set(&self, value: f64) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Gauge,
            action: MetricAction::GaugeSet(value),
            name: self.key.clone(),
        });
    }
}

impl HistogramFn for MetricHandle {
    fn record(&self, value: f64) {
        self.record_in_registry(&MetricVisitor {
            metric_type: MetricType::Histogram,
            action: MetricAction::HistRecord(value),
            name: self.key.clone(),
        });
    }
}
