use tracing::Span;

use super::{
    layer::{CapturingLayer, WithContext},
    traceparent::TraceParent,
};

pub trait SpanExt {
    fn set_trace_parent(&self, trace_parent: TraceParent);
    fn with_trace_parent(self, trace_parent: TraceParent) -> Self
    where
        Self: Sized;
}

impl SpanExt for Span {
    fn set_trace_parent(&self, trace_parent: TraceParent) {
        self.with_subscriber(|(id, dispatch)| {
            if let Some(get_context) = dispatch.downcast_ref::<WithContext>() {
                get_context.with_context(dispatch, id, |tp| {
                    *tp = Some(trace_parent);
                });
            }
        });
    }

    fn with_trace_parent(self, trace_parent: TraceParent) -> Self {
        self.set_trace_parent(trace_parent);
        self
    }
}
