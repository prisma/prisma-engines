use super::models::TraceSpan;
use once_cell::sync::Lazy;
use opentelemetry::sdk::export::trace::SpanData;
use opentelemetry::trace::{TraceContextExt, TraceId};
use opentelemetry::Context;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{Metadata, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::EnvFilter;

pub static SHOW_ALL_TRACES: Lazy<bool> = Lazy::new(|| match std::env::var("PRISMA_SHOW_ALL_TRACES") {
    Ok(enabled) => enabled.eq_ignore_ascii_case("true"),
    Err(_) => false,
});

pub fn spans_to_json(spans: Vec<SpanData>) -> String {
    let json_spans: Vec<Value> = spans.into_iter().map(|span| json!(TraceSpan::from(span))).collect();
    let span_result = json!({
        "span": true,
        "spans": json_spans
    });
    serde_json::to_string(&span_result).unwrap_or_default()
}

// set the parent context and return the traceparent
pub fn set_parent_context_from_json_str(span: &Span, trace: &str) -> Option<String> {
    let trace: HashMap<String, String> = serde_json::from_str(trace).unwrap_or_default();
    let trace_id = trace.get("traceparent").map(String::from);
    let cx = opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&trace));
    span.set_parent(cx);
    trace_id
}

pub fn set_span_link_from_traceparent(span: &Span, traceparent: Option<String>) {
    if let Some(traceparent) = traceparent {
        let trace: HashMap<String, String> = HashMap::from([("traceparent".to_string(), traceparent)]);
        let cx = opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&trace));
        let context_span = cx.span();
        span.add_link(context_span.span_context().clone());
    }
}

pub fn get_trace_parent_from_span(span: &Span) -> String {
    let cx = span.context();
    let binding = cx.span();
    let span_context = binding.span_context();

    format!("00-{}-{}-01", span_context.trace_id(), span_context.span_id())
}

pub fn get_trace_id_from_span(span: &Span) -> TraceId {
    let cx = span.context();
    get_trace_id_from_context(&cx)
}

pub fn get_trace_id_from_context(context: &Context) -> TraceId {
    let context_span = context.span();
    context_span.span_context().trace_id()
}

pub fn get_trace_id_from_traceparent(traceparent: Option<&str>) -> TraceId {
    traceparent
        .unwrap_or("0-0-0-0")
        .split('-')
        .nth(1)
        .map(|id| TraceId::from_hex(id).unwrap_or(TraceId::INVALID))
        .unwrap()
}

pub enum QueryEngineLogLevel {
    FromEnv,
    Override(String),
}

impl QueryEngineLogLevel {
    fn level(self) -> Option<String> {
        match self {
            Self::FromEnv => std::env::var("QE_LOG_LEVEL").ok(),
            Self::Override(l) => Some(l),
        }
    }
}

#[rustfmt::skip]
pub fn env_filter(log_queries: bool, qe_log_level: QueryEngineLogLevel) -> EnvFilter {
    let mut filter = EnvFilter::from_default_env()
        .add_directive("tide=error".parse().unwrap())
        .add_directive("tonic=error".parse().unwrap())
        .add_directive("h2=error".parse().unwrap())
        .add_directive("hyper=error".parse().unwrap())
        .add_directive("tower=error".parse().unwrap());

    if let Some(level) = qe_log_level.level() {
        filter = filter
            .add_directive(format!("query_engine={}", &level).parse().unwrap())
            .add_directive(format!("query_core={}", &level).parse().unwrap())
            .add_directive(format!("query_connector={}", &level).parse().unwrap())
            .add_directive(format!("sql_query_connector={}", &level).parse().unwrap())
            .add_directive(format!("mongodb_query_connector={}", &level).parse().unwrap());
    }

    if log_queries {
        filter = filter
            .add_directive("quaint[{is_query}]=trace".parse().unwrap())
            .add_directive("mongodb_query_connector=debug".parse().unwrap());
    }

    filter
}

pub fn user_facing_span_only_filter(meta: &Metadata<'_>) -> bool {
    if !meta.is_span() {
        return false;
    }

    if *SHOW_ALL_TRACES {
        return true;
    }

    if meta.fields().iter().any(|f| f.name() == "user_facing") {
        return true;
    }

    // spans describing a quaint query.
    // TODO: should this span be made user_facing in quaint?
    meta.target() == "quaint::connector::metrics" && meta.name() == "quaint:query"
}
