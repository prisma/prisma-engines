#![allow(dead_code)]

use core::fmt;
// use js_sys::Function as JsFunction;
use query_core::telemetry;
use serde_json::Value;
use std::{collections::BTreeMap, ffi::c_char};
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
// use wasm_bindgen::JsValue;

pub type CStringFunc = extern "C" fn(*const c_char);

#[derive(Clone)]
pub struct LogCallback(pub CStringFunc);

impl LogCallback {
    pub fn call<T: Into<String>>(&self, arg1: T) -> Result<(), String> {
        let message = arg1.into();
        let message = std::ffi::CString::new(message).unwrap();
        (self.0)(message.as_ptr());

        Ok(())

        // self.0
        //     .call1(&JsValue::NULL, &arg1.into())
        //     .map(|_| ())
        //     .map_err(|err| err.as_string().unwrap_or_default())
    }
}

unsafe impl Send for LogCallback {}
unsafe impl Sync for LogCallback {}

pub(crate) struct Logger {
    dispatcher: Dispatch,
}

impl Logger {
    /// Creates a new logger using a call layer
    pub fn new(log_queries: bool, log_level: LevelFilter, log_callback: LogCallback, enable_tracing: bool) -> Self {
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

        let is_user_trace = filter_fn(telemetry::helpers::user_facing_span_only_filter);
        let tracer = super::tracer::new_pipeline().install_simple(log_callback.clone());
        let telemetry = if enable_tracing {
            let telemetry = tracing_opentelemetry::layer()
                .with_tracer(tracer)
                .with_filter(is_user_trace);
            Some(telemetry)
        } else {
            None
        };

        let layer = CallbackLayer::new(log_callback).with_filter(filters);

        Self {
            dispatcher: Dispatch::new(Registry::default().with(telemetry).with(layer)),
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

impl<'a> ToString for JsonVisitor<'a> {
    fn to_string(&self) -> String {
        serde_json::to_string(&self.values).unwrap()
    }
}

pub(crate) struct CallbackLayer {
    callback: LogCallback,
}

impl CallbackLayer {
    pub fn new(callback: LogCallback) -> Self {
        CallbackLayer { callback }
    }
}

// A tracing layer for sending logs to a js callback, layers are composable, subscribers are not.
impl<S: Subscriber> Layer<S> for CallbackLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = JsonVisitor::new(event.metadata().level(), event.metadata().target());
        event.record(&mut visitor);

        let _ = self.callback.call(visitor.to_string());
    }
}
