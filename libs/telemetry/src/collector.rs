use std::{
    borrow::Cow,
    collections::HashMap,
    time::{Duration, SystemTime},
};

#[cfg(test)]
use serde::Serialize;
use tokio::time::Instant;

use crate::id::{RequestId, SpanId};
use crate::models::{LogLevel, SpanKind};

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Serialize))]
pub struct CollectedSpan {
    pub(crate) id: SpanId,
    pub(crate) parent_id: Option<SpanId>,
    pub(crate) name: Cow<'static, str>,
    #[cfg_attr(test, serde(skip_serializing))]
    pub(crate) start_time: SystemTime,
    #[cfg_attr(test, serde(skip_serializing))]
    pub(crate) duration: Duration,
    pub(crate) attributes: HashMap<&'static str, serde_json::Value>,
    pub(crate) kind: SpanKind,
    pub(crate) links: Vec<SpanId>,
}

pub(crate) struct SpanBuilder {
    request_id: Option<RequestId>,
    id: SpanId,
    name: Cow<'static, str>,
    // we store both the wall clock time and a monotonically increasing instant to
    // be resilient against clock changes between the start and end of the span
    start_time: SystemTime,
    start_instant: Instant,
    attributes: HashMap<&'static str, serde_json::Value>,
    kind: Option<SpanKind>,
    links: Vec<SpanId>,
}

impl SpanBuilder {
    pub fn new(name: &'static str, id: impl Into<SpanId>, attrs_size_hint: usize) -> Self {
        Self {
            request_id: None,
            id: id.into(),
            name: name.into(),
            start_time: SystemTime::now(),
            start_instant: Instant::now(),
            attributes: HashMap::with_capacity(attrs_size_hint),
            kind: None,
            links: Vec::new(),
        }
    }

    pub fn request_id(&self) -> Option<RequestId> {
        self.request_id
    }

    pub fn set_request_id(&mut self, request_id: RequestId) {
        self.request_id = Some(request_id);
    }

    pub fn set_name(&mut self, name: Cow<'static, str>) {
        self.name = name;
    }

    pub fn set_kind(&mut self, kind: SpanKind) {
        self.kind = Some(kind);
    }

    pub fn insert_attribute(&mut self, key: &'static str, value: serde_json::Value) {
        self.attributes.insert(key, value);
    }

    pub fn add_link(&mut self, link: SpanId) {
        self.links.push(link);
    }

    pub fn end(self, parent_id: Option<impl Into<SpanId>>) -> CollectedSpan {
        CollectedSpan {
            id: self.id,
            parent_id: parent_id.map(Into::into),
            name: self.name,
            start_time: self.start_time,
            duration: self.start_instant.elapsed(),
            attributes: self.attributes,
            kind: self.kind.unwrap_or(SpanKind::Internal),
            links: self.links,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Serialize))]
pub struct CollectedEvent {
    pub(crate) span_id: SpanId,
    pub(crate) name: &'static str,
    pub(crate) level: LogLevel,
    #[cfg_attr(test, serde(skip_serializing))]
    pub(crate) timestamp: SystemTime,
    pub(crate) attributes: HashMap<&'static str, serde_json::Value>,
}

pub(crate) struct EventBuilder {
    span_id: SpanId,
    name: &'static str,
    level: LogLevel,
    timestamp: SystemTime,
    attributes: HashMap<&'static str, serde_json::Value>,
}

impl EventBuilder {
    pub fn new(span_id: SpanId, name: &'static str, level: LogLevel, attrs_size_hint: usize) -> Self {
        Self {
            span_id,
            name,
            level,
            timestamp: SystemTime::now(),
            attributes: HashMap::with_capacity(attrs_size_hint),
        }
    }

    pub fn insert_attribute(&mut self, key: &'static str, value: serde_json::Value) {
        self.attributes.insert(key, value);
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    pub fn build(self) -> CollectedEvent {
        CollectedEvent {
            span_id: self.span_id,
            name: self.name,
            level: self.level,
            timestamp: self.timestamp,
            attributes: self.attributes,
        }
    }
}

pub trait Collector {
    fn add_span(&self, trace: RequestId, span: CollectedSpan);
    fn add_event(&self, trace: RequestId, event: CollectedEvent);
}
