use std::{borrow::Cow, collections::HashMap};

use crate::models::{HrTime, SpanKind, TraceSpan};

pub(crate) struct SpanBuilder {
    name: Cow<'static, str>,
    start_time: HrTime,
    end_time: Option<HrTime>,
    attributes: HashMap<Cow<'static, str>, serde_json::Value>,
    kind: Option<SpanKind>,
}

impl SpanBuilder {
    pub fn new(name: impl Into<Cow<'static, str>>, start_time: HrTime) -> Self {
        Self {
            name: name.into(),
            start_time,
            end_time: None,
            attributes: HashMap::new(),
            kind: None,
        }
    }

    pub fn insert_attribute(&mut self, key: impl Into<Cow<'static, str>>, value: serde_json::Value) {
        self.attributes.insert(key.into(), value);
    }

    pub fn set_kind(&mut self, kind: SpanKind) {
        self.kind = Some(kind);
    }

    pub fn end(self, end_time: HrTime) -> TraceSpan {
        TraceSpan {
            name: self.name,
            start_time: self.start_time,
            end_time,
            attributes: self.attributes,
            kind: self.kind.unwrap_or(SpanKind::Internal),
            trace_id: todo!(),
            span_id: todo!(),
            parent_span_id: todo!(),
            events: todo!(),
            links: todo!(),
        }
    }
}

pub struct Collector {}
