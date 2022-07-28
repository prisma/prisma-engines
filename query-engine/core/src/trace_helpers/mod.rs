use once_cell::sync::Lazy;
use opentelemetry::sdk::export::trace::SpanData;
use serde_json::{json, Value};
use std::borrow::Cow;

use std::{collections::HashMap, time::SystemTime};
use tracing::{Metadata, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

const ACCEPT_ATTRIBUTES: &[&str] = &["db.statement", "itx_id", "db.type"];

pub static SHOW_ALL_TRACES: Lazy<bool> = Lazy::new(|| match std::env::var("PRISMA_SHOW_ALL_TRACES") {
    Ok(enabled) => enabled.to_lowercase() == "true".to_string(),
    Err(_) => false,
});

pub fn spans_to_json(spans: &[SpanData]) -> String {
    let json_spans: Vec<Value> = spans.iter().map(span_to_json).collect();
    let span_result = json!({
        "span": true,
        "spans": json_spans
    });

    match serde_json::to_string(&span_result) {
        Ok(json_string) => json_string,
        Err(_) => "".to_string(),
    }
}

fn span_to_json(span: &SpanData) -> Value {
    let attributes: HashMap<String, String> =
        span.attributes
            .iter()
            .fold(HashMap::default(), |mut map, (key, value)| {
                if ACCEPT_ATTRIBUTES.contains(&key.as_str()) {
                    map.insert(key.to_string(), value.to_string());
                }

                map
            });

    // Override the name of quaint. It will be confusing for users to see quaint instead of
    // Prisma in the spans.
    let name: Cow<str> = match span.name {
        Cow::Borrowed("quaint:query") => "prisma:db_query".into(),
        _ => span.name.clone(),
    };

    json!({
        "span": true,
        "trace_id": span.span_context.trace_id().to_string(),
        "span_id": span.span_context.span_id().to_string(),
        "parent_span_id": span.parent_span_id.to_string(),
        "name": name,
        "start_time": span.start_time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis().to_string(),
        "end_time": span.end_time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis().to_string(),
        "attributes": attributes
    })
}

// set the parent context and return the traceparent
pub fn set_parent_context_from_json_str(span: &Span, trace: String) -> Option<String> {
    let trace: HashMap<String, String> = serde_json::from_str(&trace).unwrap_or_default();
    let trace_id = trace.get("traceparent").map(String::from);
    let cx = opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&trace));
    span.set_parent(cx);
    trace_id
}

pub fn is_user_facing_trace_filter(meta: &Metadata) -> bool {
    if !meta.is_span() {
        return false;
    }

    if *SHOW_ALL_TRACES {
        return true;
    }

    if meta.fields().iter().any(|f| f.name() == "user_facing") {
        return true;
    }

    meta.target() == "quaint::connector::metrics" && meta.name() == "quaint:query"
}
