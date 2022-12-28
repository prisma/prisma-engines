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
    let trace_id = trace.get("traceparent").map(String::from);
    let cx = opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&trace));
    span.set_parent(cx);
    trace_id
}

pub fn set_span_link_from_trace_id(span: &Span, trace_id: Option<String>) {
    if let Some(trace_id) = trace_id {
        let trace: HashMap<String, String> = HashMap::from([("traceparent".to_string(), trace_id)]);
        let cx = opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&trace));
        let context_span = cx.span();
        span.add_link(context_span.span_context().clone());
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
    fn level(self) -> String {
        match self {
            Self::FromEnv => match std::env::var("QE_LOG_LEVEL") {
                Ok(l) => l,
                _ => "error".to_string(),
            },
            Self::Override(l) => l,
        }
    }
}

#[cfg_attr(rustfmt, rustfmt_skip)]
pub fn env_filter(log_queries: bool, qe_log_level: QueryEngineLogLevel) -> EnvFilter {
    let mut filter = EnvFilter::from_default_env()
        .add_directive("tide=error".parse().unwrap())
        .add_directive("tonic=error".parse().unwrap())
        .add_directive("h2=error".parse().unwrap())
        .add_directive("hyper=error".parse().unwrap())
        .add_directive("tower=error".parse().unwrap());

    let level = qe_log_level.level();
    
    filter = filter
        .add_directive(format!("query_engine={}", &level).parse().unwrap())
        .add_directive(format!("query_core={}", &level).parse().unwrap())
        .add_directive(format!("query_connector={}", &level).parse().unwrap())
        .add_directive(format!("sql_query_connector={}", &level).parse().unwrap())
        .add_directive(format!("mongodb_query_connector={}", &level).parse().unwrap());

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
