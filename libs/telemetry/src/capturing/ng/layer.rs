use std::marker::PhantomData;

use tracing::{
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use crate::helpers::TraceParent;

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
}

impl<S> Layer<S> for CapturingLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span_scope) = ctx.span_scope(id) else {
            return;
        };

        let mut span_scope = span_scope.from_root();
        let root = span_scope
            .next()
            .expect("span scope always includes at least the span we requested the scope for");

        let Some(trace_parent) = root.extensions().get::<TraceParent>().cloned() else {
            // we don't want to collect traces not originating from client requests
            return;
        };

        if !trace_parent.sampled() {
            return;
        }

        for span in span_scope {
            span.extensions_mut().insert(trace_parent);
            dbg!(span.id().into_u64(), span.name());
        }
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
