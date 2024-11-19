use std::marker::PhantomData;

use tokio::time::Instant;
use tracing::{
    field,
    span::{Attributes, Id},
    Dispatch, Subscriber,
};
use tracing_subscriber::{
    layer::Context,
    registry::{LookupSpan, SpanRef},
    Layer,
};

use crate::models::SpanKind;

use super::{collector::SpanBuilder, traceparent::TraceParent};

const SPAN_NAME_FIELD: &str = "otel.name";
const SPAN_KIND_FIELD: &str = "otel.kind";

pub fn layer<S>() -> CapturingLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    CapturingLayer::default()
}

pub struct CapturingLayer<S> {
    _registry: PhantomData<S>,
}

impl<S> Default for CapturingLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn default() -> Self {
        CapturingLayer::new()
    }
}

impl<S> CapturingLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    pub fn new() -> Self {
        Self { _registry: PhantomData }
    }

    fn root_span_checked<'a>(id: &Id, ctx: &'a Context<'a, S>) -> Option<SpanRef<'a, S>> {
        ctx.span_scope(id)?.from_root().next()
    }

    fn root_span<'a>(id: &Id, ctx: &'a Context<'a, S>) -> SpanRef<'a, S> {
        Self::root_span_checked(id, ctx)
            .expect("span scope must exist in the current subscriber and include at least the requested span ID")
    }
}

impl<S> Layer<S> for CapturingLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let span_builder = SpanBuilder::new(span.name(), id.to_owned(), Instant::now(), attrs.fields().len());

        span.extensions_mut().insert(span_builder);
    }

    fn on_record(&self, _span: &Id, _values: &tracing::span::Record<'_>, _ctx: Context<'_, S>) {}

    fn on_follows_from(&self, _span: &Id, _follows: &Id, _ctx: Context<'_, S>) {}

    fn event_enabled(&self, _event: &tracing::Event<'_>, _ctx: Context<'_, S>) -> bool {
        true
    }

    fn on_event(&self, _event: &tracing::Event<'_>, _ctx: Context<'_, S>) {}

    fn on_enter(&self, _id: &Id, _ctx: Context<'_, S>) {}

    fn on_exit(&self, _id: &Id, _ctx: Context<'_, S>) {}

    fn on_close(&self, _id: Id, _ctx: Context<'_, S>) {}

    fn on_id_change(&self, _old: &Id, _new: &Id, _ctx: Context<'_, S>) {}
}

struct SpanAttributeVisitor<'a> {
    span_builder: &'a mut SpanBuilder,
}

impl<'a> SpanAttributeVisitor<'a> {
    fn new(span_builder: &'a mut SpanBuilder) -> Self {
        Self { span_builder }
    }
}

impl<'a> field::Visit for SpanAttributeVisitor<'a> {
    fn record_f64(&mut self, field: &field::Field, value: f64) {
        self.span_builder.insert_attribute(field.name(), value.into())
    }

    fn record_i64(&mut self, field: &field::Field, value: i64) {
        self.span_builder.insert_attribute(field.name(), value.into())
    }

    fn record_u64(&mut self, field: &field::Field, value: u64) {
        self.span_builder.insert_attribute(field.name(), value.into())
    }

    fn record_bool(&mut self, field: &field::Field, value: bool) {
        self.span_builder.insert_attribute(field.name(), value.into())
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        match field.name() {
            SPAN_NAME_FIELD => self.span_builder.set_name(value.to_owned().into()),
            SPAN_KIND_FIELD => self.span_builder.set_kind(value.parse().unwrap_or(SpanKind::Internal)),
            _ => self.span_builder.insert_attribute(field.name(), value.into()),
        }
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
        self.record_str(field, &format!("{:?}", value))
    }
}
