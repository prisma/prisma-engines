use core::fmt;
use js_sys::Function as JsFunction;
use query_engine_common::logger::StringCallback;
use serde_json::Value;
use std::collections::BTreeMap;
use telemetry::Exporter;
use tracing::{
    Dispatch, Level, Subscriber,
    field::{Field, Visit},
    level_filters::LevelFilter,
};
use tracing_subscriber::{
    Layer, Registry,
    filter::{FilterExt, filter_fn},
    layer::SubscriberExt,
};
use wasm_bindgen::JsValue;

#[derive(Clone)]
pub struct LogCallback(pub JsFunction);

unsafe impl Send for LogCallback {}
unsafe impl Sync for LogCallback {}

pub(crate) struct Logger {
    dispatcher: Dispatch,
    exporter: Exporter,
}

impl Logger {
    /// Creates a new logger using a call layer
    pub fn new(log_queries: bool, log_level: LevelFilter, log_callback: LogCallback, enable_tracing: bool) -> Self {
        let is_sql_query = filter_fn(|meta| {
            meta.target() == "quaint::connector::trace" && meta.fields().iter().any(|f| f.name() == "query")
        });

        // We need to filter the messages to send to our callback logging mechanism
        let filters = if log_queries {
            // Filter trace query events (for query log) or based in the defined log level
            is_sql_query.or(log_level).boxed()
        } else {
            // Filter based in the defined log level
            FilterExt::boxed(log_level)
        };

        let log_layer = CallbackLayer::new(log_callback).with_filter(filters);

        let exporter = Exporter::new();

        let tracing_layer = enable_tracing
            .then(|| telemetry::layer(exporter.clone()).with_filter(telemetry::filter::user_facing_spans()));

        Self {
            dispatcher: Dispatch::new(Registry::default().with(tracing_layer).with(log_layer)),
            exporter,
        }
    }

    pub fn dispatcher(&self) -> Dispatch {
        self.dispatcher.clone()
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

impl std::fmt::Display for JsonVisitor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        self.callback
            .0
            .call1(&JsValue::NULL, &message.into())
            .map(|_| ())
            .map_err(|err| format!("Could not call JS callback: {}", err.as_string().unwrap_or_default()))
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
