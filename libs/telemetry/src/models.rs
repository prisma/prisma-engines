use std::{
    borrow::Cow,
    collections::HashMap,
    str::FromStr,
    time::{Duration, SystemTime},
};

use enumflags2::bitflags;
use opentelemetry::{sdk::export::trace::SpanData, KeyValue, Value};
use serde::Serialize;
use serde_json::json;

const ACCEPT_ATTRIBUTES: &[&str] = &[
    "db.system",
    "db.statement",
    "db.collection.name",
    "db.operation.name",
    "itx_id",
];

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[bitflags]
#[repr(u8)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Query,
}

impl From<tracing::Level> for LogLevel {
    fn from(value: tracing::Level) -> Self {
        match value {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        }
    }
}

impl From<&tracing::Level> for LogLevel {
    fn from(value: &tracing::Level) -> Self {
        Self::from(*value)
    }
}

impl FromStr for LogLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "query" => Ok(LogLevel::Query),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum SpanKind {
    #[serde(rename = "client")]
    Client,
    #[serde(rename = "internal")]
    Internal,
}

impl FromStr for SpanKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "client" => Ok(SpanKind::Client),
            "internal" => Ok(SpanKind::Internal),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TraceSpan {
    pub(super) trace_id: String,
    pub(super) span_id: String,
    pub(super) parent_span_id: String,
    pub(super) name: Cow<'static, str>,
    pub(super) start_time: HrTime,
    pub(super) end_time: HrTime,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(super) attributes: HashMap<Cow<'static, str>, serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) events: Vec<Event>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) links: Vec<Link>,
    pub(super) kind: SpanKind,
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
        let kind = match span.span_kind {
            opentelemetry::trace::SpanKind::Client => SpanKind::Client,
            _ => SpanKind::Internal,
        };

        let attributes: HashMap<Cow<'static, str>, serde_json::Value> =
            span.attributes
                .iter()
                .fold(HashMap::default(), |mut map, (key, value)| {
                    if ACCEPT_ATTRIBUTES.contains(&key.as_str()) {
                        map.insert(key.to_string().into(), to_json_value(value));
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
            name,
            start_time: hr_start_time,
            end_time: hr_end_time,
            attributes,
            links,
            events,
            kind,
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

/// High-resolution time in the same format that OpenTelemetry uses.
///
/// The first number is Unix time in seconds since 00:00:00 UTC on 1 January 1970.
/// The second number is the sub-second amount of time elapsed since time represented by the first
/// number in nanoseconds.
///
/// ## Example
///
/// For example, `2021-01-01T12:30:10.150Z` in Unix time in milliseconds is 1609504210150.
/// Then the first number can be calculated by converting and truncating the epoch time in
/// milliseconds to seconds:
///
/// ```js
/// time[0] = Math.trunc(1609504210150 / 1000) // = 1609504210
/// ```
///
/// The second number can be calculated by converting the digits after the decimal point of the
/// expression `(1609504210150 / 1000) - time[0]` to nanoseconds:
///
/// ```js
/// time[1] = Number((1609504210.150 - time[0]).toFixed(9)) * 1e9 // = 150000000.
/// ```
///
/// Therefore, this time is represented in `HrTime` format as `[1609504210, 150000000]`.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct HrTime(u64, u32);

impl From<Duration> for HrTime {
    fn from(time: Duration) -> Self {
        Self(time.as_secs(), time.subsec_nanos())
    }
}

impl From<SystemTime> for HrTime {
    fn from(time: SystemTime) -> Self {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .expect("time can't be before unix epoch")
            .into()
    }
}

fn convert_to_high_res_time(time: Duration) -> HrTime {
    let secs = time.as_secs();
    let partial = time.subsec_nanos();
    HrTime(secs, partial)
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
        assert_eq!(HrTime::from(time_val), HrTime(1609504210, 150000000));
    }
}
