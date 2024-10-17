use opentelemetry::{sdk::export::trace::SpanData, Key, KeyValue, Value};
use serde::Serialize;
use serde_json::json;
use std::{
    borrow::Cow,
    collections::HashMap,
    time::{Duration, SystemTime},
};

const ACCEPT_ATTRIBUTES: &[&str] = &["db.system", "db.statement", "itx_id", "otel.kind"];

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum OtelKind {
    #[serde(rename = "client")]
    Client,
    #[serde(rename = "internal")]
    Internal,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TraceSpan {
    pub(super) trace_id: String,
    pub(super) span_id: String,
    pub(super) parent_span_id: String,
    pub(super) name: String,
    pub(super) start_time: HrTime,
    pub(super) end_time: HrTime,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(super) attributes: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) events: Vec<Event>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) links: Vec<Link>,
    pub(super) otel_kind: OtelKind,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Link {
    trace_id: String,
    span_id: String,
}

impl TraceSpan {
    pub fn split_events(self) -> (Vec<Event>, TraceSpan) {
        (self.events, Self { events: vec![], ..self })
    }
}

impl From<SpanData> for TraceSpan {
    fn from(span: SpanData) -> Self {
        let otel_kind = match span.attributes.get(&Key::from_static_str("otel.kind")) {
            Some(Value::String(kind)) => match kind {
                Cow::Borrowed("client") => OtelKind::Client,
                _ => OtelKind::Internal,
            },
            _ => OtelKind::Internal,
        };

        let attributes: HashMap<String, serde_json::Value> =
            span.attributes
                .iter()
                .fold(HashMap::default(), |mut map, (key, value)| {
                    if ACCEPT_ATTRIBUTES.contains(&key.as_str()) {
                        map.insert(key.to_string(), to_json_value(value));
                    }

                    map
                });

        // TODO(fernandez@prisma.io) mongo query events and quaint query events
        // have different attributes. both of them are queries, however the name
        // of quaint queries is quaint::query and the name of mongodb queries is
        // prisma::engine::db_query. Both of them are generated as spans but quaint
        // contains the full query, while mongodb only contains the collection name
        // and the operatiion. For this reason, we treat them differently when geneating
        // query events in logging capturing and other places.
        //
        // What we are currently doing is to add a quaint attribute to quaint queries
        // so we can transform span containing the query into a query event. For mongo
        // this is not enough and we need to extract a particular event.
        //
        // If we unified these two ways of logging / tracing query information, we could get rid of
        // a lot of spaghetti code.

        let is_quaint_query = matches!(span.name, Cow::Borrowed("quaint:query"));

        let name: Cow<'_, str> = if is_quaint_query {
            "prisma:engine:db_query".into()
        } else {
            span.name.clone()
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

        let span_id = span.span_context.span_id().to_string();
        let events = span
            .events
            .into_iter()
            .map(|e| Event::from(e).with_span_id(span_id.clone()))
            .collect();

        Self {
            span_id,
            trace_id: span.span_context.trace_id().to_string(),
            parent_span_id: span.parent_span_id.to_string(),
            name: name.into_owned(),
            start_time: hr_start_time,
            end_time: hr_end_time,
            attributes,
            links,
            events,
            otel_kind,
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub(super) span_id: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(super) name: String,
    pub(super) level: String,
    pub(super) timestamp: HrTime,
    pub(super) attributes: HashMap<String, serde_json::Value>,
}

impl Event {
    pub(super) fn with_span_id(mut self, spain_id: String) -> Self {
        self.span_id = Some(spain_id);
        self
    }
}

impl From<opentelemetry::trace::Event> for Event {
    fn from(event: opentelemetry::trace::Event) -> Self {
        let name = event.name.to_string();
        let timestamp = convert_to_high_res_time(event.timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap());
        let mut attributes: HashMap<String, serde_json::Value> =
            event
                .attributes
                .into_iter()
                .fold(Default::default(), |mut map, KeyValue { key, value }| {
                    map.insert(key.to_string(), to_json_value(&value));
                    map
                });

        let level = attributes
            .remove("level")
            .unwrap_or_else(|| serde_json::Value::String("unknown".to_owned()))
            .to_string()
            .to_ascii_lowercase();

        Self {
            span_id: None, // already attached to the span
            name,
            level,
            timestamp,
            attributes,
        }
    }
}
/// logs are modeled as span events
pub type LogEvent = Event;
/// metrics are modeled as span events
pub type MetricEvent = Event;

pub type HrTime = [u64; 2];

///  Take from the otel library on what the format should be for High-Resolution time
///  Defines High-Resolution Time.
///
///  The first number, HrTime[0], is UNIX Epoch time in seconds since 00:00:00 UTC on 1 January 1970.
///  The second number, HrTime[1], represents the partial second elapsed since Unix Epoch time represented by first number in nanoseconds.
///  For example, 2021-01-01T12:30:10.150Z in UNIX Epoch time in milliseconds is represented as 1609504210150.
///  The first number is calculated by converting and truncating the Epoch time in milliseconds to seconds:
/// HrTime[0] = Math.trunc(1609504210150 / 1000) = 1609504210.
/// The second number is calculated by converting the digits after the decimal point of the subtraction, (1609504210150 / 1000) - HrTime[0], to nanoseconds:
/// HrTime[1] = Number((1609504210.150 - HrTime[0]).toFixed(9)) * 1e9 = 150000000.
/// This is represented in HrTime format as [1609504210, 150000000].
fn convert_to_high_res_time(time: Duration) -> HrTime {
    let secs = time.as_secs();
    let partial = time.subsec_nanos();
    [secs, partial as u64]
}

/// Transforms an [`opentelemetry::Value`] to a [`serde_json::Value`]
/// This is because we want to flatten the JSON representation for a value, which by default will
/// be a nested structure informing of the type. For instance a float [`opentelemetry::Value`]
/// would be represented as json as `{"f64": 1.0}`. This function will flatten it to just `1.0`.
fn to_json_value(value: &Value) -> serde_json::Value {
    match value {
        Value::String(s) => json!(s),
        Value::F64(f) => json!(f),
        Value::Bool(b) => json!(b),
        Value::I64(i) => json!(i),
        Value::Array(ary) => match ary {
            opentelemetry::Array::Bool(b_vec) => json!(b_vec),
            opentelemetry::Array::I64(i_vec) => json!(i_vec),
            opentelemetry::Array::F64(f_vec) => json!(f_vec),
            opentelemetry::Array::String(s_vec) => json!(s_vec),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_resolution_time_works() {
        // 2021-01-01T12:30:10.150Z in UNIX Epoch time in milliseconds
        let time_val = Duration::from_millis(1609504210150);
        assert_eq!([1609504210, 150000000], convert_to_high_res_time(time_val));
    }
}
