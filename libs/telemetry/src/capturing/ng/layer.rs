use std::marker::PhantomData;

use tracing::{
    span::{Attributes, Id},
    Dispatch, Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

use super::traceparent::TraceParent;

pub fn layer<S>() -> CapturingLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    CapturingLayer::default()
}

pub struct CapturingLayer<S> {
    _registry: PhantomData<S>,
    get_context: WithContext,
}

/// We can't easily downcast `Subscriber` to a specific layer type without knowing the concrete
/// type of `S`. This function remembers the type of the subscriber so we have something else
/// non-generic to downcast to in `SpanExt`.
///
/// This is a common and idiomatic pattern in the `tracing` ecosystem, see for example:
/// - https://github.com/tokio-rs/tracing/blob/2ea8f8cc509300f193811a63f7270cfcaa81bc22/tracing-error/src/layer.rs#L29-L34
/// - https://github.com/tokio-rs/tracing-opentelemetry/blob/f6fc075fe0095ee9a7363c8b67818d160f869c48/src/layer.rs#L79-L87
pub(crate) struct WithContext(fn(&Dispatch, &Id, &mut dyn FnMut(&mut Option<TraceParent>)));

impl WithContext {
    pub(crate) fn with_context(&self, dispatch: &Dispatch, id: &Id, mut f: impl FnMut(&mut Option<TraceParent>)) {
        (self.0)(dispatch, id, &mut f)
    }
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
        Self {
            _registry: PhantomData,
            get_context: WithContext(Self::get_context),
        }
    }

    fn get_context(dispatch: &Dispatch, id: &Id, f: &mut dyn FnMut(&mut Option<TraceParent>)) {
        let registry = dispatch
            .downcast_ref::<S>()
            .expect("dispatch should be related to a subscriber with the expected type");

        let span = registry
            .span(id)
            .expect("registry should have a span with the specified ID");

        let mut extensions = span.extensions_mut();

        if let Some(trace_parent) = extensions.get_mut::<Option<TraceParent>>() {
            f(trace_parent);
        } else {
            let mut new_trace_parent = None;
            f(&mut new_trace_parent);
            extensions.insert(new_trace_parent);
        }
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

        // let Some(trace_parent) = root.extensions().get::<TraceParent>().cloned() else {
        //     // we don't want to collect traces not originating from client requests
        //     return;
        // };

        let trace_parent = root.extensions().get::<Option<TraceParent>>().cloned().flatten();

        if let Some(trace_parent) = trace_parent {
            // if !trace_parent.sampled() {
            //     return;
            // }

            for span in span_scope {
                // span.extensions_mut().insert(trace_parent);
                span.extensions_mut().insert(Some(trace_parent));
                dbg!(span.id().into_u64(), span.name());
            }
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
