use std::{borrow::Cow, collections::HashMap, num::NonZeroU64};

use serde::Serialize;
use tokio::time::Instant;
use tracing::Level;

use crate::models::{HrTime, SpanKind, TraceSpan};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct SpanId(NonZeroU64);

impl From<&tracing::span::Id> for SpanId {
    fn from(id: &tracing::span::Id) -> Self {
        Self(id.into_non_zero_u64())
    }
}

impl From<tracing::span::Id> for SpanId {
    fn from(id: tracing::span::Id) -> Self {
        Self::from(&id)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct CollectedSpan {
    id: SpanId,
    parent_id: Option<SpanId>,
    name: Cow<'static, str>,
    #[cfg_attr(test, serde(skip_serializing))]
    start_time: Instant,
    #[cfg_attr(test, serde(skip_serializing))]
    end_time: Instant,
    attributes: HashMap<&'static str, serde_json::Value>,
    kind: SpanKind,
    links: Vec<SpanId>,
}

pub(crate) struct SpanBuilder {
    id: SpanId,
    name: Cow<'static, str>,
    start_time: Instant,
    end_time: Option<Instant>,
    attributes: HashMap<&'static str, serde_json::Value>,
    kind: Option<SpanKind>,
    links: Vec<SpanId>,
}

impl SpanBuilder {
    pub fn new(name: &'static str, id: impl Into<SpanId>, start_time: Instant, attrs_size_hint: usize) -> Self {
        Self {
            id: id.into(),
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

    pub fn add_link(&mut self, link: SpanId) {
        self.links.push(link);
    }

    pub fn end(self, parent_id: Option<impl Into<SpanId>>, end_time: Instant) -> CollectedSpan {
        CollectedSpan {
            id: self.id,
            parent_id: parent_id.map(Into::into),
            name: self.name,
            start_time: self.start_time,
            end_time,
            attributes: self.attributes,
            kind: self.kind.unwrap_or(SpanKind::Internal),
            links: self.links,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CollectedEvent {
    name: &'static str,
    level: Level,
    timestamp: Instant,
    attributes: HashMap<&'static str, serde_json::Value>,
}

pub trait Collector {
    fn add_span(&self, trace: SpanId, span: CollectedSpan);
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
    fn add_span(&self, _trace: SpanId, _span: CollectedSpan) {
        todo!()
    }
}
