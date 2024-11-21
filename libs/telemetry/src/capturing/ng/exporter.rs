use std::{collections::HashMap, sync::Arc};

use super::collector::{CollectedEvent, CollectedSpan, Collector, RequestId, SpanId};

#[derive(Clone)]
pub struct Exporter(Arc<ExporterInner>);

struct ExporterInner {
    tasks: HashMap<SpanId, ()>,
}

impl Exporter {
    pub fn new() -> Self {
        Self(Arc::new(ExporterInner { tasks: HashMap::new() }))
    }
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for Exporter {
    fn add_span(&self, _trace: RequestId, _span: CollectedSpan) {
        todo!()
    }

    fn add_event(&self, _trace: RequestId, _event: CollectedEvent) {
        todo!()
    }
}
