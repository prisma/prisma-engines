use super::models::TraceSpan;
use opentelemetry::sdk::export::trace::SpanData;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::Metadata;
use tracing_subscriber::EnvFilter;

pub static SHOW_ALL_TRACES: LazyLock<bool> = LazyLock::new(|| match std::env::var("PRISMA_SHOW_ALL_TRACES") {
    Ok(enabled) => enabled.eq_ignore_ascii_case("true"),
    Err(_) => false,
});

pub use crate::capturing::ng::traceparent::TraceParent;

pub fn spans_to_json(spans: Vec<SpanData>) -> String {
    let json_spans: Vec<Value> = spans.into_iter().map(|span| json!(TraceSpan::from(span))).collect();
    let span_result = json!({
        "span": true,
        "spans": json_spans
    });
    serde_json::to_string(&span_result).unwrap_or_default()
}

pub fn restore_remote_context_from_json_str(serialized: &str) -> opentelemetry::Context {
    // This relies on the fact that global text map propagator was installed that
    // can handle `traceparent` field (for example, `TraceContextPropagator`).
    let trace: HashMap<String, String> = serde_json::from_str(serialized).unwrap_or_default();
    opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&trace))
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

pub fn env_filter(log_queries: bool, qe_log_level: QueryEngineLogLevel) -> EnvFilter {
    let mut filter = EnvFilter::from_default_env()
        .add_directive("tonic=error".parse().unwrap())
        .add_directive("h2=error".parse().unwrap())
        .add_directive("hyper=error".parse().unwrap())
        .add_directive("tower=error".parse().unwrap());

    if let Some(ref level) = qe_log_level.level() {
        filter = filter
            .add_directive(format!("query_engine={}", level).parse().unwrap())
            .add_directive(format!("query_core={}", level).parse().unwrap())
            .add_directive(format!("query_connector={}", level).parse().unwrap())
            .add_directive(format!("sql_query_connector={}", level).parse().unwrap())
            .add_directive(format!("mongodb_query_connector={}", level).parse().unwrap());
    }

    if log_queries {
        filter = filter
            .add_directive("quaint[{is_query}]=trace".parse().unwrap())
            .add_directive("mongodb_query_connector[{is_query}]=debug".parse().unwrap());
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
