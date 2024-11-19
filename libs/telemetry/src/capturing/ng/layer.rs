use std::marker::PhantomData;

use tokio::time::Instant;
use tracing::{
    field,
    span::{Attributes, Id},
    Dispatch, Subscriber,
};
use tracing_subscriber::{
    layer::Context,
    registry::{LookupSpan, SpanRef},
    Layer,
};

use crate::models::SpanKind;

use super::{
    collector::{Collector, Exporter, SpanBuilder},
    traceparent::TraceParent,
};

const SPAN_NAME_FIELD: &str = "otel.name";
const SPAN_KIND_FIELD: &str = "otel.kind";

pub fn layer<S, C>(collector: C) -> CapturingLayer<S, C>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    C: Collector + 'static,
{
    CapturingLayer::new(collector)
}

pub struct CapturingLayer<S, C> {
    _registry: PhantomData<S>,
    collector: C,
}

impl<S, C> CapturingLayer<S, C>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    C: Collector + 'static,
{
    pub fn new(collector: C) -> Self {
        Self {
            _registry: PhantomData,
            collector,
        }
    }

    fn require_span<'a>(id: &Id, ctx: &'a Context<'_, S>) -> SpanRef<'a, S> {
        ctx.span(id).expect("span must exist in the registry, this is a bug")
    }

    fn root_span_checked<'a>(id: &Id, ctx: &'a Context<'_, S>) -> Option<SpanRef<'a, S>> {
        ctx.span_scope(id)?.from_root().next()
    }

    fn root_span<'a>(id: &Id, ctx: &'a Context<'_, S>) -> SpanRef<'a, S> {
        Self::root_span_checked(id, ctx)
            .expect("span scope must exist in the registry and include at least the requested span ID")
    }
}

impl<S, C> Layer<S> for CapturingLayer<S, C>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    C: Collector + 'static,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = Self::require_span(id, &ctx);
        let mut span_builder = SpanBuilder::new(span.name(), id, Instant::now(), attrs.fields().len());

        attrs.record(&mut SpanAttributeVisitor::new(&mut span_builder));

        span.extensions_mut().insert(span_builder);
    }

    fn on_record(&self, span: &Id, values: &tracing::span::Record<'_>, ctx: Context<'_, S>) {
        let span = Self::require_span(span, &ctx);
        let mut extensions = span.extensions_mut();

        if let Some(span_builder) = extensions.get_mut::<SpanBuilder>() {
            values.record(&mut SpanAttributeVisitor::new(span_builder));
        }
    }

    fn on_follows_from(&self, span: &Id, follows: &Id, ctx: Context<'_, S>) {
        let span = Self::require_span(span, &ctx);
        let mut extensions = span.extensions_mut();

        if let Some(span_builder) = extensions.get_mut::<SpanBuilder>() {
            span_builder.add_link(follows.into());
        }
    }

    fn on_event(&self, _event: &tracing::Event<'_>, _ctx: Context<'_, S>) {}

    fn on_enter(&self, _id: &Id, _ctx: Context<'_, S>) {}

    fn on_exit(&self, _id: &Id, _ctx: Context<'_, S>) {}

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = Self::require_span(&id, &ctx);

        let Some(span_builder) = span.extensions_mut().remove::<SpanBuilder>() else {
            return;
        };

        let end_time = Instant::now();
        let parent_id = span.parent().map(|parent| parent.id());
        let collected_span = span_builder.end(parent_id, end_time);

        let trace_id = Self::root_span(&id, &ctx).id();

        self.collector.add_span(trace_id.into(), collected_span);
    }
}

struct SpanAttributeVisitor<'a> {
    span_builder: &'a mut SpanBuilder,
}

impl<'a> SpanAttributeVisitor<'a> {
    fn new(span_builder: &'a mut SpanBuilder) -> Self {
        Self { span_builder }
    }
}

impl<'a> field::Visit for SpanAttributeVisitor<'a> {
    fn record_f64(&mut self, field: &field::Field, value: f64) {
        self.span_builder.insert_attribute(field.name(), value.into())
    }

    fn record_i64(&mut self, field: &field::Field, value: i64) {
        self.span_builder.insert_attribute(field.name(), value.into())
    }

    fn record_u64(&mut self, field: &field::Field, value: u64) {
        self.span_builder.insert_attribute(field.name(), value.into())
    }

    fn record_bool(&mut self, field: &field::Field, value: bool) {
        self.span_builder.insert_attribute(field.name(), value.into())
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        match field.name() {
            SPAN_NAME_FIELD => self.span_builder.set_name(value.to_owned().into()),
            SPAN_KIND_FIELD => self.span_builder.set_kind(value.parse().unwrap_or(SpanKind::Internal)),
            _ => self.span_builder.insert_attribute(field.name(), value.into()),
        }
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
        self.record_str(field, &format!("{:?}", value))
    }
}

#[cfg(test)]
mod tests {
    use crate::capturing::ng::collector::{CollectedSpan, SpanId};

    use super::*;

    use std::cell::RefCell;
    use std::collections::{BTreeMap, HashMap};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, LazyLock, Mutex};
    use std::time::Duration;

    use insta::assert_ron_snapshot;
    use insta::internals::{Content, Redaction};
    use tracing::{info_span, span, Level};
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    #[derive(Debug, Default, Clone)]
    struct TestCollector {
        spans: Arc<Mutex<BTreeMap<SpanId, Vec<CollectedSpan>>>>,
    }

    impl TestCollector {
        fn new() -> Self {
            Self::default()
        }

        fn get_spans(&self) -> BTreeMap<SpanId, Vec<CollectedSpan>> {
            self.spans.lock().unwrap().clone()
        }
    }

    impl Collector for TestCollector {
        fn add_span(&self, trace_id: SpanId, span: CollectedSpan) {
            let mut spans = self.spans.lock().unwrap();
            spans.entry(trace_id).or_default().push(span);
        }
    }

    fn redact_id() -> Redaction {
        thread_local! {
            static SPAN_ID_TO_SEQUENTIAL_ID: RefCell<HashMap<u64, u64>> = <_>::default();
            static NEXT_ID: RefCell<u64> = const { RefCell::new(1) };
        }

        fn redact_recursive(value: Content) -> Content {
            match value {
                Content::NewtypeStruct("SpanId", ref nested) => match **nested {
                    Content::U64(original_id) => SPAN_ID_TO_SEQUENTIAL_ID.with_borrow_mut(|map| {
                        let id = map.entry(original_id).or_insert_with(|| {
                            NEXT_ID.with_borrow_mut(|next_id| {
                                let id = *next_id;
                                *next_id += 1;
                                id
                            })
                        });
                        Content::NewtypeStruct("SpanId", Box::new(Content::U64(*id)))
                    }),
                    _ => value,
                },
                Content::Some(nested) => Content::Some(Box::new(redact_recursive(*nested))),
                _ => value,
            }
        }

        insta::dynamic_redaction(|value, _path| redact_recursive(value))
    }

    #[test]
    fn test_basic_span_collection() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let span = info_span!("test_span", otel.kind = "client");
            let _guard = span.enter();
        });

        let spans = collector.get_spans();

        assert_ron_snapshot!(
            spans,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          SpanId(1): [
            CollectedSpan(
              id: SpanId(1),
              parent_id: None,
              name: "test_span",
              attributes: {},
              kind: client,
              links: [],
            ),
          ],
        }
        "#
        );
    }

    #[test]
    fn test_nested_spans() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let parent = info_span!("parent_span");
            let _parent_guard = parent.enter();

            {
                let child = info_span!("child_span", otel.kind = "internal");
                let _child_guard = child.enter();
                std::thread::sleep(Duration::from_millis(10));
            }
        });

        let spans = collector.get_spans();

        assert_ron_snapshot!(
            spans,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          SpanId(1): [
            CollectedSpan(
              id: SpanId(2),
              parent_id: Some(SpanId(1)),
              name: "child_span",
              attributes: {},
              kind: internal,
              links: [],
            ),
            CollectedSpan(
              id: SpanId(1),
              parent_id: None,
              name: "parent_span",
              attributes: {},
              kind: internal,
              links: [],
            ),
          ],
        }
        "#
        );
    }

    #[test]
    fn test_span_attributes() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let span = info_span!(
                "attribute_span",
                otel.kind = "client",
                string_attr = "value",
                bool_attr = true,
                int_attr = 42,
                float_attr = 3.5
            );
            let _guard = span.enter();
        });

        let spans = collector.get_spans();

        assert_ron_snapshot!(
            spans,
            {
                ".*" => redact_id(),
                ".*[].**" => redact_id(),
                ".*[].attributes" => insta::sorted_redaction()
            },
            @r#"
        {
          SpanId(1): [
            CollectedSpan(
              id: SpanId(1),
              parent_id: None,
              name: "attribute_span",
              attributes: {
                "bool_attr": true,
                "float_attr": 3.5,
                "int_attr": 42,
                "string_attr": "value",
              },
              kind: client,
              links: [],
            ),
          ],
        }
        "#
        );
    }

    #[test]
    fn test_span_updates() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let span = info_span!("updated_span", otel.kind = "client");
            span.record("dynamic_attr", "added_later");
            let _guard = span.enter();
        });

        let spans = collector.get_spans();

        assert_ron_snapshot!(
            spans,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          SpanId(1): [
            CollectedSpan(
              id: SpanId(1),
              parent_id: None,
              name: "updated_span",
              attributes: {},
              kind: client,
              links: [],
            ),
          ],
        }
        "#
        );
    }
}
