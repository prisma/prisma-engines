use std::{borrow::Cow, collections::HashMap};

use tokio::time::Instant;
use tracing::{span::Id, Level};

use crate::models::{HrTime, SpanKind, TraceSpan};

#[derive(Debug)]
pub struct CollectedSpan {
    id: Id,
    parent_id: Option<Id>,
    name: Cow<'static, str>,
    start_time: Instant,
    end_time: Instant,
    attributes: HashMap<&'static str, serde_json::Value>,
    kind: SpanKind,
    links: Vec<Id>,
}

pub(crate) struct SpanBuilder {
    id: Id,
    name: Cow<'static, str>,
    start_time: Instant,
    end_time: Option<Instant>,
    attributes: HashMap<&'static str, serde_json::Value>,
    kind: Option<SpanKind>,
    links: Vec<Id>,
}

impl SpanBuilder {
    pub fn new(name: &'static str, id: Id, start_time: Instant, attrs_size_hint: usize) -> Self {
        Self {
            id,
            name: name.into(),
            start_time,
            end_time: None,
            attributes: HashMap::with_capacity(attrs_size_hint),
            kind: None,
            links: Vec::new(),
        }
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

    pub fn add_link(&mut self, link: Id) {
        self.links.push(link);
    }

    pub fn end(self, parent_id: Option<Id>, end_time: Instant) -> CollectedSpan {
        CollectedSpan {
            id: self.id,
            parent_id,
            name: self.name,
            start_time: self.start_time,
            end_time,
            attributes: self.attributes,
            kind: self.kind.unwrap_or(SpanKind::Internal),
            links: self.links,
        }
    }
}

#[derive(Debug)]
pub(crate) struct CollectedEvent {
    name: &'static str,
    level: Level,
    timestamp: Instant,
    attributes: HashMap<&'static str, serde_json::Value>,
}

pub trait Collector {
    fn add_span(&self, trace: Id, span: CollectedSpan);
}

pub struct Exporter {}

impl Exporter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for Exporter {
    fn add_span(&self, _trace: Id, _span: CollectedSpan) {
        todo!()
    }
}
