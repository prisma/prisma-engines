use super::visitor::JsonVisitor;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use serde_json::{Map, Value};
use tracing::{metadata::LevelFilter, span::Record, Event, Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

pub struct EventChannel {
    callback: ThreadsafeFunction<String>,
    level_filter: LevelFilter,
}

impl Clone for EventChannel {
    fn clone(&self) -> Self {
        Self {
            level_filter: self.level_filter,
            // we crash if javascript aborts the callback, but continues using
            // the engine.
            callback: self.callback.try_clone().unwrap(),
        }
    }
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
    fn new_span(&self, attrs: &tracing::span::Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).unwrap();

        let mut extensions = span.extensions_mut();

        if extensions.get_mut::<Map<String, Value>>().is_none() {
            let mut object = Map::with_capacity(10);
            let mut visitor = JsonVisitor::new(&mut object);
            attrs.record(&mut visitor);
            extensions.insert(object);
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).unwrap();
        let mut extensions = span.extensions_mut();

        if let Some(mut object) = extensions.get_mut::<Map<String, Value>>() {
            let mut visitor = JsonVisitor::new(&mut object);

            values.record(&mut visitor);
        } else {
            let mut object = Map::with_capacity(10);
            let mut visitor = JsonVisitor::new(&mut object);

            values.record(&mut visitor);
            extensions.insert(object);
        }
    }

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

        self.callback.call(Ok(json_str), ThreadsafeFunctionCallMode::Blocking);
    }

    fn enabled(&self, metadata: &tracing::Metadata<'_>, ctx: Context<'_, S>) -> bool {
        self.level_filter.enabled(metadata, ctx)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(self.level_filter)
    }
}
