use super::models::TraceSpan;
use once_cell::sync::Lazy;
use opentelemetry::sdk::export::trace::SpanData;
use opentelemetry::trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState};
use opentelemetry::Context;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{Metadata, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::EnvFilter;

pub static SHOW_ALL_TRACES: Lazy<bool> = Lazy::new(|| match std::env::var("PRISMA_SHOW_ALL_TRACES") {
    Ok(enabled) => enabled.to_lowercase() == *("true"),
    Err(_) => false,
});

pub fn spans_to_json(spans: Vec<SpanData>) -> String {
    let json_spans: Vec<Value> = spans.into_iter().map(|span| json!(TraceSpan::from(span))).collect();
    let span_result = json!({
        "span": true,
        "spans": json_spans
    });

    match serde_json::to_string(&span_result) {
        Ok(json_string) => json_string,
        Err(_) => "".to_string(),
    }
}

// set the parent context and return the traceparent
pub fn set_parent_context_from_json_str(span: &Span, trace: &str) -> Option<String> {
    let trace: HashMap<String, String> = serde_json::from_str(trace).unwrap_or_default();
    // TODO: miguelff investigate this, I think this is wrong, a traceparent is more than a traceid
    // but this code asumes, a traceid is the string representation of a traceparent, and that's used
    // in the node api
    let trace_id = trace.get("traceparent").map(String::from);
    let cx = opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&trace));
    span.set_parent(cx);
    trace_id
}

pub fn set_span_link_from_trace_id(span: &Span, trace_id: Option<String>) {
    if let Some(trace_id) = trace_id {
        // TODO: miguelff. Investigate how the previous (wrong implementation) was silently broken
        // this is a hack to get a link to the trace_id of an operation. Before this hack, this was
        // wrong, links were not properly because it was trying to create a link assuming the traceparent
        // was a traceid, thus leading to a wrong context.
        let sc = SpanContext::new(
            TraceId::from_hex(&trace_id).unwrap_or(TraceId::INVALID),
            SpanId::from_hex("1").unwrap_or(SpanId::INVALID),
            TraceFlags::default(),
            false,
            TraceState::default(),
        );
        span.add_link(sc);
    }
}

pub fn get_trace_id_from_context(context: &Context) -> TraceId {
    let context_span = context.span();
    context_span.span_context().trace_id()
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

pub fn user_facing_span_only_filter(meta: &Metadata) -> bool {
    if !meta.is_span() {
        return false;
    }

    user_facing_filter(meta)
}

pub fn user_facing_filter(meta: &Metadata) -> bool {
    if *SHOW_ALL_TRACES {
        return true;
    }

    if meta.fields().iter().any(|f| f.name() == "user_facing") {
        return true;
    }

    meta.target() == "quaint::connector::metrics" && meta.name() == "quaint:query"
}
