use super::models::TraceSpan;
use derive_more::Display;
use once_cell::sync::Lazy;
use opentelemetry::propagation::Extractor;
use opentelemetry::sdk::export::trace::SpanData;
use opentelemetry::trace::{SpanId, TraceContextExt, TraceFlags, TraceId};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::Metadata;
use tracing_subscriber::EnvFilter;

pub static SHOW_ALL_TRACES: Lazy<bool> = Lazy::new(|| match std::env::var("PRISMA_SHOW_ALL_TRACES") {
    Ok(enabled) => enabled.eq_ignore_ascii_case("true"),
    Err(_) => false,
});

/// Traceparent is a remote span. It is identified by trace_id and span_id.
///
/// By "remote" we mean that this span was not emitted in the current process. In real life, it is
/// either:
///  - Emitted by the JS part of the Prisma ORM. This is true both for Accelerate (where the Rust
///    part is deployed as a server) and for the ORM (where the Rust part is a shared library)
///  - Never emitted at all. This happens when the `TraceParent` is created artificially from `TxId`
///    (see `TxId::as_traceparent`). In this case, `TraceParent` is used only to correlate logs
///    from different transaction operations - it is never used as a part of the trace
#[derive(Display, Copy, Clone)]
// This conforms with https://www.w3.org/TR/trace-context/#traceparent-header-field-values. Accelerate
// relies on this behaviour.
#[display(fmt = "00-{trace_id:032x}-{span_id:016x}-{flags:02x}")]
pub struct TraceParent {
    trace_id: TraceId,
    span_id: SpanId,
    flags: TraceFlags,
}

impl TraceParent {
    pub fn from_remote_context(context: &opentelemetry::Context) -> Option<Self> {
        let span = context.span();
        let span_context = span.span_context();

        if span_context.is_valid() {
            Some(Self {
                trace_id: span_context.trace_id(),
                span_id: span_context.span_id(),
                flags: span_context.trace_flags(),
            })
        } else {
            None
        }
    }

    // TODO(aqrln): remove this method once the log capturing doesn't rely on trace IDs anymore
    #[deprecated = "this must only be used to create an artificial traceparent for log capturing when tracing is disabled on the client"]
    pub fn new_random() -> Self {
        Self {
            trace_id: TraceId::from_bytes(rand::random()),
            span_id: SpanId::from_bytes(rand::random()),
            flags: TraceFlags::SAMPLED,
        }
    }

    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    pub fn sampled(&self) -> bool {
        self.flags.is_sampled()
    }

    /// Returns a remote `opentelemetry::Context`. By "remote" we mean that it wasn't emitted in the
    /// current process.
    pub fn to_remote_context(&self) -> opentelemetry::Context {
        // This relies on the fact that global text map propagator was installed that
        // can handle `traceparent` field (for example, `TraceContextPropagator`).
        opentelemetry::global::get_text_map_propagator(|propagator| {
            propagator.extract(&TraceParentExtractor::new(self))
        })
    }
}

/// An extractor to use with `TraceContextPropagator`. It allows to avoid creating a full `HashMap`
/// to convert a `TraceParent` to a `Context`.
pub struct TraceParentExtractor(String);

impl TraceParentExtractor {
    pub fn new(traceparent: &TraceParent) -> Self {
        Self(traceparent.to_string())
    }
}

impl Extractor for TraceParentExtractor {
    fn get(&self, key: &str) -> Option<&str> {
        if key == "traceparent" {
            Some(&self.0)
        } else {
            None
        }
    }

    fn keys(&self) -> Vec<&str> {
        vec!["traceparent"]
    }
}

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

#[rustfmt::skip]
pub fn env_filter(log_queries: bool, qe_log_level: QueryEngineLogLevel) -> EnvFilter {
    let mut filter = EnvFilter::from_default_env()
        .add_directive("tide=error".parse().unwrap())
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
