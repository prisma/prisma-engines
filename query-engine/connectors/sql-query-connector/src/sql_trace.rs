use opentelemetry::trace::{SpanContext, TraceContextExt, TraceFlags};
use quaint::ast::{Delete, Insert, Select, Update};
use telemetry::helpers::TraceParent;
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub fn trace_parent_to_string(context: &SpanContext) -> String {
    let trace_id = context.trace_id();
    let span_id = context.span_id();

    // see https://www.w3.org/TR/trace-context/#traceparent-header-field-values
    format!("traceparent='00-{trace_id:032x}-{span_id:016x}-01'")
}

pub trait SqlTraceComment: Sized {
    fn append_trace(self, span: &Span) -> Self;
    fn add_traceparent(self, traceparent: Option<TraceParent>) -> Self;
}

macro_rules! sql_trace {
    ($what:ty) => {
        impl SqlTraceComment for $what {
            fn append_trace(self, span: &Span) -> Self {
                let otel_ctx = span.context();
                let span_ref = otel_ctx.span();
                let otel_ctx = span_ref.span_context();

                if otel_ctx.trace_flags() == TraceFlags::SAMPLED {
                    self.comment(trace_parent_to_string(otel_ctx))
                } else {
                    self
                }
            }

            // Temporary method to pass the traceid in an operation
            fn add_traceparent(self, traceparent: Option<TraceParent>) -> Self {
                let Some(traceparent) = traceparent else {
                    return self;
                };

                if traceparent.sampled() {
                    self.comment(format!("traceparent='{}'", traceparent))
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
