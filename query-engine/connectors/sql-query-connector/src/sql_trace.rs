use opentelemetry::trace::{SpanContext, TraceContextExt};
use quaint::ast::{Delete, Insert, Select, Update};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub fn trace_parent_to_string(context: &SpanContext) -> String {
    let trace_id = context.trace_id().to_hex();
    let span_id = context.span_id().to_hex();

    // see https://www.w3.org/TR/trace-context/#traceparent-header-field-values
    format!("traceparent=00-{}-{}-01", trace_id, span_id)
}

pub trait SqlTraceComment: Sized {
    fn append_trace(self, span: &Span) -> Self;
}

macro_rules! sql_trace {
    ($what:ty) => {
        impl SqlTraceComment for $what {
            fn append_trace(self, span: &Span) -> Self {
                let span_ctx = span.context();
                let otel_ctx = span_ctx.span().span_context();

                if otel_ctx.trace_flags() == 1 {
                    self.comment(trace_parent_to_string(otel_ctx))
                } else {
                    self
                }
            }
        }
    };
}

sql_trace!(Insert<'_>);

sql_trace!(Update<'_>);

sql_trace!(Delete<'_>);

sql_trace!(Select<'_>);
