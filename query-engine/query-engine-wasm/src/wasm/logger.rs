#![allow(dead_code)]

use core::fmt;
use js_sys::Function as JsFunction;
use query_engine_common::logger::StringCallback;
use serde_json::Value;
use std::collections::BTreeMap;
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
use wasm_bindgen::JsValue;

#[derive(Clone)]
pub struct LogCallback(pub JsFunction);

unsafe impl Send for LogCallback {}
unsafe impl Sync for LogCallback {}

pub(crate) struct Logger {
    dispatcher: Dispatch,
}

impl Logger {
    /// Creates a new logger using a call layer
    pub fn new(log_queries: bool, log_level: LevelFilter, log_callback: LogCallback) -> Self {
        let is_sql_query = filter_fn(|meta| {
            meta.target() == "quaint::connector::metrics" && meta.fields().iter().any(|f| f.name() == "query")
        });

        // We need to filter the messages to send to our callback logging mechanism
        let filters = if log_queries {
            // Filter trace query events (for query log) or based in the defined log level
            is_sql_query.or(log_level).boxed()
        } else {
            // Filter based in the defined log level
            FilterExt::boxed(log_level)
        };

        let log_callback = CallbackLayer::new(log_callback);
        let layer = log_callback.with_filter(filters);

        Self {
            dispatcher: Dispatch::new(Registry::default().with(layer)),
        }
    }

    pub fn dispatcher(&self) -> Dispatch {
        self.dispatcher.clone()
    }
}

pub struct JsonVisitor<'a> {
    values: BTreeMap<&'a str, Value>,
}

impl<'a> JsonVisitor<'a> {
    pub fn new(level: &Level, target: &str) -> Self {
        let mut values = BTreeMap::new();
        values.insert("level", serde_json::Value::from(level.to_string()));

        // NOTE: previous version used module_path, this is not correct and it should be _target_
        values.insert("module_path", serde_json::Value::from(target));

        JsonVisitor { values }
    }
}

impl<'a> Visit for JsonVisitor<'a> {
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

impl<'a> std::fmt::Display for JsonVisitor<'a> {
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
