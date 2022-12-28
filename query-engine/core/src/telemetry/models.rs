use opentelemetry::{sdk::export::trace::SpanData, KeyValue};
use serde::Serialize;
use std::{
    borrow::Cow,
    collections::HashMap,
    time::{Duration, SystemTime},
};

const ACCEPT_ATTRIBUTES: &[&str] = &["db.statement", "itx_id", "db.type"];

pub type HrTime = [u64; 2];

#[derive(Serialize, Debug, Clone)]
pub struct TraceSpan {
    pub(self) trace_id: String,
    pub(self) span_id: String,
    pub(self) parent_span_id: String,
    pub(self) name: String,
    pub(self) start_time: HrTime,
    pub(self) end_time: HrTime,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(self) attributes: HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(self) events: Vec<Event>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(self) links: Vec<Link>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Link {
    trace_id: String,
    span_id: String,
}

impl TraceSpan {
    pub(super) fn is_query(&self) -> bool {
        self.name.eq("prisma:engine:db_query")
    }

    pub(super) fn query_event(&self) -> Event {
        Event {
            span_id: Some(self.span_id.to_owned()),
            name: self.attributes.get("db.statement").unwrap().to_string(),
            level: "query".to_string(),
            timestamp: self.start_time,
            attributes: Default::default(),
        }
    }

    pub fn split_logs(self) -> (Vec<Event>, TraceSpan) {
        (self.events, Self { events: vec![], ..self })
    }
}

impl From<SpanData> for TraceSpan {
    fn from(span: SpanData) -> Self {
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
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Event {
    pub(super) span_id: Option<String>,
    pub(super) name: String,
    pub(super) level: String,
    pub(super) timestamp: HrTime,
    pub(super) attributes: HashMap<String, String>,
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
        let mut attributes: HashMap<String, String> =
            event
                .attributes
                .iter()
                .fold(Default::default(), |mut map, KeyValue { key, value }| {
                    map.insert(key.to_string(), value.clone().to_string());
                    map
                });

        let level = attributes
            .remove("level")
            .unwrap_or_else(|| "unknown".to_string())
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
