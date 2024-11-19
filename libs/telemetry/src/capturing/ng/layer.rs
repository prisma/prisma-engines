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

use super::{
    collector::{Collector, Exporter, SpanBuilder},
    traceparent::TraceParent,
};

const SPAN_NAME_FIELD: &str = "otel.name";
const SPAN_KIND_FIELD: &str = "otel.kind";

pub fn layer<S, C>(collector: C) -> CapturingLayer<S, C>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    C: Collector + 'static,
{
    CapturingLayer::new(collector)
}

pub struct CapturingLayer<S, C> {
    _registry: PhantomData<S>,
    collector: C,
}

impl<S, C> CapturingLayer<S, C>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    C: Collector + 'static,
{
    pub fn new(collector: C) -> Self {
        Self {
            _registry: PhantomData,
            collector,
        }
    }

    fn require_span<'a>(id: &Id, ctx: &'a Context<'_, S>) -> SpanRef<'a, S> {
        ctx.span(id).expect("span must exist in the registry, this is a bug")
    }

    fn root_span_checked<'a>(id: &Id, ctx: &'a Context<'_, S>) -> Option<SpanRef<'a, S>> {
        ctx.span_scope(id)?.from_root().next()
    }

    fn root_span<'a>(id: &Id, ctx: &'a Context<'_, S>) -> SpanRef<'a, S> {
        Self::root_span_checked(id, ctx)
            .expect("span scope must exist in the registry and include at least the requested span ID")
    }
}

impl<S, C> Layer<S> for CapturingLayer<S, C>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    C: Collector + 'static,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = Self::require_span(id, &ctx);
        let mut span_builder = SpanBuilder::new(span.name(), id.to_owned(), Instant::now(), attrs.fields().len());

        attrs.record(&mut SpanAttributeVisitor::new(&mut span_builder));

        span.extensions_mut().insert(span_builder);
    }

    fn on_record(&self, span: &Id, values: &tracing::span::Record<'_>, ctx: Context<'_, S>) {
        let span = Self::require_span(span, &ctx);
        let mut extensions = span.extensions_mut();

        if let Some(span_builder) = extensions.get_mut::<SpanBuilder>() {
            values.record(&mut SpanAttributeVisitor::new(span_builder));
        }
    }

    fn on_follows_from(&self, span: &Id, follows: &Id, ctx: Context<'_, S>) {
        let span = Self::require_span(span, &ctx);
        let mut extensions = span.extensions_mut();

        if let Some(span_builder) = extensions.get_mut::<SpanBuilder>() {
            span_builder.add_link(follows.to_owned());
        }
    }

    fn on_event(&self, _event: &tracing::Event<'_>, _ctx: Context<'_, S>) {}

    fn on_enter(&self, _id: &Id, _ctx: Context<'_, S>) {}

    fn on_exit(&self, _id: &Id, _ctx: Context<'_, S>) {}

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = Self::require_span(&id, &ctx);
        let mut extensions = span.extensions_mut();

        if let Some(span_builder) = extensions.remove::<SpanBuilder>() {
            let end_time = Instant::now();
            let parent_id = span.parent().map(|parent| parent.id());
            let collected_span = span_builder.end(parent_id, end_time);

            let trace_id = Self::root_span(&id, &ctx).id();

            self.collector.add_span(trace_id, collected_span);
        }
    }
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
