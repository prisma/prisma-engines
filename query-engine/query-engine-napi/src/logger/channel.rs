use super::visitor::JsonVisitor;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use serde_json::{Map, Value};
use tracing::{metadata::LevelFilter, Event, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

#[derive(Clone)]
pub struct EventChannel {
    callback: ThreadsafeFunction<String>,
    level_filter: LevelFilter,
}

impl EventChannel {
    pub fn new(callback: ThreadsafeFunction<String>) -> Self {
        Self {
            callback,
            level_filter: LevelFilter::OFF,
        }
    }

    pub fn filter_level(&mut self, level_filter: LevelFilter) {
        self.level_filter = level_filter;
    }
}

impl<S> Layer<S> for EventChannel
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut object: Map<String, Value> = Map::with_capacity(5);

        object.insert("level".to_string(), format!("{}", event.metadata().level()).into());

        let metadata = event.metadata();
        if let Some(module_path) = metadata.module_path() {
            object.insert("module_path".to_string(), module_path.into());
        }

        let mut visitor = JsonVisitor::new(&mut object);
        event.record(&mut visitor);

        let js_object = Value::Object(object);
        let json_str = serde_json::to_string(&js_object).unwrap();

        self.callback
            .call(Ok(json_str), ThreadsafeFunctionCallMode::NonBlocking);
    }

    fn enabled(&self, metadata: &tracing::Metadata<'_>, ctx: Context<'_, S>) -> bool {
        self.level_filter.enabled(metadata, ctx)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(self.level_filter)
    }
}
