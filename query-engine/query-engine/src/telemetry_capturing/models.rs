use opentelemetry::{sdk::export::trace::SpanData, trace::Event, KeyValue, Value};
use query_core::convert_to_high_res_time;
use serde::Serialize;
use std::{borrow::Cow, collections::HashMap, time::SystemTime};

const ACCEPT_ATTRIBUTES: &[&str] = &["db.statement", "itx_id", "db.type"];

#[derive(Serialize, Debug, Clone)]
pub struct ExportedSpan {
    trace_id: String,
    span_id: String,
    parent_span_id: String,
    name: String,
    start_time: [u64; 2],
    end_time: [u64; 2],
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    attributes: HashMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    events: Vec<ExportedEvent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    links: Vec<Link>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Link {
    trace_id: String,
    span_id: String,
}

impl From<SpanData> for ExportedSpan {
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

        dbg!(span.events.clone());
        println!("\n\n\n\n\n\n\n");

        let events = span
            .events
            .into_iter()
            .map(|event| ExportedEvent::from(event))
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
            events,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ExportedEvent {
    pub name: String,
    pub timestamp: [u64; 2],
    pub attributes: HashMap<String, Value>,
}

impl From<Event> for ExportedEvent {
    fn from(event: Event) -> Self {
        let name = event.name.to_string();
        let timestamp = convert_to_high_res_time(event.timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap());
        let attributes: HashMap<String, Value> =
            event
                .attributes
                .iter()
                .fold(Default::default(), |mut map, KeyValue { key, value }| {
                    map.insert(key.to_string(), value.clone());
                    map
                });

        Self {
            name,
            timestamp,
            attributes,
        }
    }
}
