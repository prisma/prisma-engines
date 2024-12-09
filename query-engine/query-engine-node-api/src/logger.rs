use core::fmt;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};
use prisma_metrics::{MetricRecorder, MetricRegistry};
use query_engine_common::logger::StringCallback;
use serde_json::Value;
use std::{collections::BTreeMap, fmt::Display};
use telemetry::Exporter;
use tracing::{
    field::{Field, Visit},
    level_filters::LevelFilter,
    Dispatch, Level, Subscriber,
};
use tracing_subscriber::{
    filter::{filter_fn, FilterExt},
    layer::SubscriberExt,
    Layer, Registry,
};

pub(crate) type LogCallback = ThreadsafeFunction<String, ErrorStrategy::Fatal>;

pub(crate) struct Logger {
    dispatcher: Dispatch,
    metrics: Option<MetricRegistry>,
    recorder: Option<MetricRecorder>,
    exporter: Exporter,
}

impl Logger {
    /// Creates a new logger using a call layer
    pub fn new(
        log_queries: bool,
        log_level: LevelFilter,
        log_callback: LogCallback,
        enable_metrics: bool,
        enable_tracing: bool,
    ) -> Self {
        let is_sql_query = filter_fn(|meta| {
            meta.target() == "quaint::connector::metrics" && meta.fields().iter().any(|f| f.name() == "query")
        });

        // is a mongodb query?
        let is_mongo_query = filter_fn(|meta| meta.target() == "mongodb_query_connector::query");

        // We need to filter the messages to send to our callback logging mechanism
        let filters = if log_queries {
            // Filter trace query events (for query log) or based in the defined log level
            is_sql_query.or(is_mongo_query).or(log_level).boxed()
        } else {
            // Filter based in the defined log level
            FilterExt::boxed(log_level)
        };

        let log_callback = CallbackLayer::new(log_callback);

        let exporter = Exporter::new();

        let telemetry = enable_tracing
            .then(|| telemetry::layer(exporter.clone()).with_filter(telemetry::filter::user_facing_spans()));

        let layer = log_callback.with_filter(filters);

        let (metrics, recorder) = if enable_metrics {
            let registry = MetricRegistry::new();
            let recorder = MetricRecorder::new(registry.clone()).with_initialized_prisma_metrics();
            (Some(registry), Some(recorder))
        } else {
            (None, None)
        };

        Self {
            dispatcher: Dispatch::new(Registry::default().with(telemetry).with(layer)),
            metrics,
            recorder,
            exporter,
        }
    }

    pub fn dispatcher(&self) -> Dispatch {
        self.dispatcher.clone()
    }

    pub fn metrics(&self) -> Option<MetricRegistry> {
        self.metrics.clone()
    }

    pub fn recorder(&self) -> Option<MetricRecorder> {
        self.recorder.clone()
    }

    pub fn exporter(&self) -> Exporter {
        self.exporter.clone()
    }
}

pub struct JsonVisitor<'a> {
    values: BTreeMap<&'a str, Value>,
}

impl JsonVisitor<'_> {
    pub fn new(level: &Level, target: &str) -> Self {
        let mut values = BTreeMap::new();
        values.insert("level", serde_json::Value::from(level.to_string()));

        // NOTE: previous version used module_path, this is not correct and it should be _target_
        values.insert("module_path", serde_json::Value::from(target));

        JsonVisitor { values }
    }
}

impl Visit for JsonVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        match field.name() {
            name if name.starts_with("r#") => {
                self.values
                    .insert(&name[2..], serde_json::Value::from(format!("{value:?}")));
            }
            name => {
                self.values.insert(name, serde_json::Value::from(format!("{value:?}")));
            }
        };
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.values.insert(field.name(), serde_json::Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.values.insert(field.name(), serde_json::Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.values.insert(field.name(), serde_json::Value::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.values.insert(field.name(), serde_json::Value::from(value));
    }
}

impl Display for JsonVisitor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string(&self.values).unwrap())
    }
}

#[derive(Clone)]
pub(crate) struct CallbackLayer {
    callback: LogCallback,
}

impl CallbackLayer {
    pub fn new(callback: LogCallback) -> Self {
        CallbackLayer { callback }
    }
}

impl StringCallback for CallbackLayer {
    fn call(&self, message: String) -> Result<(), String> {
        let status = self.callback.call(message, ThreadsafeFunctionCallMode::Blocking);

        if status != napi::Status::Ok {
            Err(format!("Could not call JS callback: {}", status))
        } else {
            Ok(())
        }
    }
}

// A tracing layer for sending logs to a js callback, layers are composable, subscribers are not.
impl<S: Subscriber> Layer<S> for CallbackLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = JsonVisitor::new(event.metadata().level(), event.metadata().target());
        event.record(&mut visitor);

        let _ = self.call(visitor.to_string());
    }
}
