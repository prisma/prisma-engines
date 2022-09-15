use opentelemetry::trace::{SpanContext, TraceContextExt, TraceFlags};
use quaint::ast::{Delete, Insert, Select, Update};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub fn trace_parent_to_string(context: &SpanContext) -> String {
    let trace_id = context.trace_id();
    let span_id = context.span_id();

    // see https://www.w3.org/TR/trace-context/#traceparent-header-field-values
    format!("traceparent=00-{:032x}-{:032x}-01", trace_id, span_id)
}

pub trait SqlTraceComment: Sized {
    fn append_trace(self, span: &Span) -> Self;
    fn add_trace_id(self, trace_id: Option<String>) -> Self;
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
            fn add_trace_id(self, trace_id: Option<String>) -> Self {
                if let Some(traceparent) = trace_id {
                    if should_sample(&traceparent) {
                        self.comment(format!("traceparent={}", traceparent))
                    } else {
                        self
                    }
                } else {
                    self
                }
            }
        }
    };
}

fn should_sample(traceparent: &str) -> bool {
    traceparent.split('-').count() == 4 && traceparent.ends_with("-01")
}

sql_trace!(Insert<'_>);

sql_trace!(Update<'_>);

sql_trace!(Delete<'_>);

sql_trace!(Select<'_>);
