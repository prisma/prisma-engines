use once_cell::sync::Lazy;
use opentelemetry::sdk::export::trace::SpanData;
use opentelemetry::trace::{TraceContextExt, TraceId};
use opentelemetry::Context;
use serde::Serialize;
use serde_json::{json, Value};
use std::borrow::Cow;

use std::time::Duration;
use std::{collections::HashMap, time::SystemTime};
use tracing::{Metadata, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

const ACCEPT_ATTRIBUTES: &[&str] = &["db.statement", "itx_id", "db.type"];

pub static SHOW_ALL_TRACES: Lazy<bool> = Lazy::new(|| match std::env::var("PRISMA_SHOW_ALL_TRACES") {
    Ok(enabled) => enabled.to_lowercase() == *("true"),
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
    json!(CapturedLog::from(span))
}

#[derive(Serialize, Debug, Clone)]
pub struct CapturedLog {
    trace_id: String,
    span_id: String,
    parent_span_id: String,
    name: String,
    start_time: [u64; 2],
    end_time: [u64; 2],
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    attributes: HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    links: Vec<Link>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Link {
    trace_id: String,
    span_id: String,
}

impl From<&SpanData> for CapturedLog {
    fn from(span: &SpanData) -> Self {
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
            Cow::Borrowed("quaint:query") => "prisma:engine:db_query".into(),
            _ => span.name.clone(),
        };

        let hr_start_time = convert_to_high_res_time(span.start_time.duration_since(SystemTime::UNIX_EPOCH).unwrap());
        let hr_end_time = convert_to_high_res_time(span.end_time.duration_since(SystemTime::UNIX_EPOCH).unwrap());

        let links = span
            .links
            .iter()
            .map(|link| {
                let ctx = link.span_context();
                Link {
                    trace_id: ctx.trace_id().to_string(),
                    span_id: ctx.span_id().to_string(),
                }
            })
            .collect();

        Self {
            trace_id: span.span_context.trace_id().to_string(),
            span_id: span.span_context.span_id().to_string(),
            parent_span_id: span.parent_span_id.to_string(),
            name: name.into_owned(),
            start_time: hr_start_time,
            end_time: hr_end_time,
            attributes,
            links,
        }
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

/**
 * Take from the otel library on what the format should be for High-Resolution time
 * Defines High-Resolution Time.
 *
 * The first number, HrTime[0], is UNIX Epoch time in seconds since 00:00:00 UTC on 1 January 1970.
 * The second number, HrTime[1], represents the partial second elapsed since Unix Epoch time represented by first number in nanoseconds.
 * For example, 2021-01-01T12:30:10.150Z in UNIX Epoch time in milliseconds is represented as 1609504210150.
 * The first number is calculated by converting and truncating the Epoch time in milliseconds to seconds:
 * HrTime[0] = Math.trunc(1609504210150 / 1000) = 1609504210.
 * The second number is calculated by converting the digits after the decimal point of the subtraction, (1609504210150 / 1000) - HrTime[0], to nanoseconds:
 * HrTime[1] = Number((1609504210.150 - HrTime[0]).toFixed(9)) * 1e9 = 150000000.
 * This is represented in HrTime format as [1609504210, 150000000].
 */
type HrTime = [u64; 2];
pub fn convert_to_high_res_time(time: Duration) -> HrTime {
    let secs = time.as_secs();
    let partial = time.subsec_nanos();
    [secs, partial as u64]
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_high_resolution_time_works() {
        // 2021-01-01T12:30:10.150Z in UNIX Epoch time in milliseconds
        let time_val = Duration::from_millis(1609504210150);
        assert_eq!([1609504210, 150000000], convert_to_high_res_time(time_val));
    }
}
