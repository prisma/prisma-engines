use std::marker::PhantomData;

use tracing::{
    field,
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{
    layer::Context,
    registry::{LookupSpan, SpanRef},
    Layer,
};

use crate::collector::{AllowAttribute, Collector, EventBuilder, SpanBuilder};
use crate::id::RequestId;
use crate::models::{LogLevel, SpanKind};

const REQUEST_ID_FIELD: &str = "request_id";
const SPAN_NAME_FIELD: &str = "otel.name";
const SPAN_KIND_FIELD: &str = "otel.kind";
const EVENT_LEVEL_FIELD: &str = "item_type";

/// Creates a new [`CapturingLayer`].
pub fn layer<S, C>(collector: C) -> CapturingLayer<S, C>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    C: Collector + 'static,
{
    CapturingLayer::new(collector)
}

/// A [`Layer`] that captures spans and events and forwards them to a [`Collector`].
///
/// This layer supports certain transformations based on the attributes of spans and events:
///
/// - The `otel.name` attribute is used to rename spans.
/// - The `otel.kind` attribute is used to set the OpenTelemetry kind of a span.
/// - The `item_type` attribute is used to override the level of an event (this is used for our
///   artificial "query" level).
///
/// Only events nested within spans are captured here. The reason for this is because we only need
/// to use the capturing mechanism for events to enable logs in response for Accelerate, and events
/// without parent spans cannot be associated with any specific client request. When the client has
/// direct access to the engine, all logs are sent directly in real time instead.
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
}

impl<S, C> Layer<S> for CapturingLayer<S, C>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    C: Collector + 'static,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = Self::require_span(id, &ctx);
        let mut span_builder = SpanBuilder::new(span.name(), id, attrs.fields().len());

        if let Some(request_id) = span
            .parent()
            .and_then(|parent| parent.extensions().get::<SpanBuilder>().and_then(|sb| sb.request_id()))
        {
            span_builder.set_request_id(request_id);
        }

        attrs.record(&mut SpanAttributeVisitor::<'_, C::AttributeFilter>::new(
            &mut span_builder,
        ));

        span.extensions_mut().insert(span_builder);
    }

    fn on_record(&self, span: &Id, values: &tracing::span::Record<'_>, ctx: Context<'_, S>) {
        let span = Self::require_span(span, &ctx);
        let mut extensions = span.extensions_mut();

        if let Some(span_builder) = extensions.get_mut::<SpanBuilder>() {
            values.record(&mut SpanAttributeVisitor::<'_, C::AttributeFilter>::new(span_builder));
        }
    }

    fn on_follows_from(&self, span: &Id, follows: &Id, ctx: Context<'_, S>) {
        let span = Self::require_span(span, &ctx);
        let mut extensions = span.extensions_mut();

        if let Some(span_builder) = extensions.get_mut::<SpanBuilder>() {
            span_builder.add_link(follows.into());
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        let Some(parent) = event.parent().cloned().or_else(|| {
            event
                .is_contextual()
                .then(|| ctx.current_span().id().cloned())
                .flatten()
        }) else {
            // Events without a parent span are not collected.
            return;
        };

        let Some(request_id) = Self::require_span(&parent, &ctx)
            .extensions()
            .get::<SpanBuilder>()
            .and_then(|sb| sb.request_id())
        else {
            return;
        };

        let mut event_builder = EventBuilder::new(
            parent.into(),
            event.metadata().name(),
            event.metadata().level().into(),
            event.metadata().fields().len(),
        );

        event.record(&mut EventAttributeVisitor::<'_, C::AttributeFilter>::new(
            &mut event_builder,
        ));

        self.collector.add_event(request_id, event_builder.build());
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = Self::require_span(&id, &ctx);

        let Some(span_builder) = span.extensions_mut().remove::<SpanBuilder>() else {
            return;
        };

        let Some(request_id) = span_builder.request_id() else {
            return;
        };

        let parent_id = span.parent().map(|parent| parent.id());
        let collected_span = span_builder.end(parent_id);

        self.collector.add_span(request_id, collected_span);
    }
}

struct SpanAttributeVisitor<'a, F> {
    span_builder: &'a mut SpanBuilder,
    _filter: PhantomData<F>,
}

impl<'a, F> SpanAttributeVisitor<'a, F> {
    fn new(span_builder: &'a mut SpanBuilder) -> Self {
        Self {
            span_builder,
            _filter: PhantomData,
        }
    }
}

impl<F: AllowAttribute> field::Visit for SpanAttributeVisitor<'_, F> {
    fn record_f64(&mut self, field: &field::Field, value: f64) {
        if F::allow_on_span(field.name()) {
            self.span_builder.insert_attribute(field.name(), value.into())
        }
    }

    fn record_i64(&mut self, field: &field::Field, value: i64) {
        if F::allow_on_span(field.name()) {
            self.span_builder.insert_attribute(field.name(), value.into())
        }
    }

    fn record_u64(&mut self, field: &field::Field, value: u64) {
        match field.name() {
            REQUEST_ID_FIELD => {
                if let Some(request_id) = RequestId::from_u64(value) {
                    self.span_builder.set_request_id(request_id);
                }
            }
            _ => {
                if F::allow_on_event(field.name()) {
                    self.span_builder.insert_attribute(field.name(), value.into())
                }
            }
        }
    }

    fn record_bool(&mut self, field: &field::Field, value: bool) {
        if F::allow_on_span(field.name()) {
            self.span_builder.insert_attribute(field.name(), value.into())
        }
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        match field.name() {
            SPAN_NAME_FIELD => self.span_builder.set_name(value.to_owned().into()),
            SPAN_KIND_FIELD => self.span_builder.set_kind(value.parse().unwrap_or(SpanKind::Internal)),
            _ => {
                if F::allow_on_span(field.name()) {
                    self.span_builder.insert_attribute(field.name(), value.into())
                }
            }
        }
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
        if F::allow_on_span(field.name()) {
            self.record_str(field, &format!("{:?}", value))
        }
    }
}

struct EventAttributeVisitor<'a, F> {
    event_builder: &'a mut EventBuilder,
    _filter: PhantomData<F>,
}

impl<'a, F> EventAttributeVisitor<'a, F> {
    fn new(event_builder: &'a mut EventBuilder) -> Self {
        Self {
            event_builder,
            _filter: PhantomData,
        }
    }
}

impl<F: AllowAttribute> field::Visit for EventAttributeVisitor<'_, F> {
    fn record_f64(&mut self, field: &field::Field, value: f64) {
        if F::allow_on_event(field.name()) {
            self.event_builder.insert_attribute(field.name(), value.into())
        }
    }

    fn record_i64(&mut self, field: &field::Field, value: i64) {
        if F::allow_on_event(field.name()) {
            self.event_builder.insert_attribute(field.name(), value.into())
        }
    }

    fn record_u64(&mut self, field: &field::Field, value: u64) {
        if F::allow_on_event(field.name()) {
            self.event_builder.insert_attribute(field.name(), value.into())
        }
    }

    fn record_bool(&mut self, field: &field::Field, value: bool) {
        if F::allow_on_event(field.name()) {
            self.event_builder.insert_attribute(field.name(), value.into())
        }
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        match field.name() {
            EVENT_LEVEL_FIELD => self.event_builder.set_level(value.parse().unwrap_or(LogLevel::Trace)),
            _ => {
                if F::allow_on_event(field.name()) {
                    self.event_builder.insert_attribute(field.name(), value.into())
                }
            }
        }
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn std::fmt::Debug) {
        if F::allow_on_event(field.name()) {
            self.record_str(field, &format!("{:?}", value))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::collector::{AllowAttribute, CollectedEvent, CollectedSpan};
    use crate::id::RequestId;

    use super::*;

    use std::cell::RefCell;
    use std::collections::{BTreeMap, HashMap};
    use std::sync::{Arc, Mutex};
    use std::thread::LocalKey;

    use insta::assert_ron_snapshot;
    use insta::internals::{Content, Redaction};
    use tracing::info_span;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    #[derive(Debug, Default, Clone)]
    struct TestCollector {
        spans: Arc<Mutex<BTreeMap<RequestId, Vec<CollectedSpan>>>>,
        events: Arc<Mutex<BTreeMap<RequestId, Vec<CollectedEvent>>>>,
    }

    impl TestCollector {
        fn new() -> Self {
            Self::default()
        }

        fn spans(&self) -> BTreeMap<RequestId, Vec<CollectedSpan>> {
            self.spans.lock().unwrap().clone()
        }

        fn events(&self) -> BTreeMap<RequestId, Vec<CollectedEvent>> {
            self.events.lock().unwrap().clone()
        }
    }

    impl Collector for TestCollector {
        type AttributeFilter = TestAttributeFilter;

        fn add_span(&self, trace_id: RequestId, span: CollectedSpan) {
            let mut spans = self.spans.lock().unwrap();
            spans.entry(trace_id).or_default().push(span);
        }

        fn add_event(&self, trace_id: RequestId, event: CollectedEvent) {
            let mut events = self.events.lock().unwrap();
            events.entry(trace_id).or_default().push(event);
        }
    }

    struct TestAttributeFilter;

    impl AllowAttribute for TestAttributeFilter {
        fn allow_on_span(name: &'static str) -> bool {
            name != "test_ignored_span_attr"
        }

        fn allow_on_event(name: &'static str) -> bool {
            name != "test_ignored_event_attr"
        }
    }

    /// Redacts span and request IDs to make snapshots stable. Mappings from original IDs to
    /// redacted IDs are stored in a thread-local hash maps, which ensures each test gets their own
    /// namespace of IDs (as libtest runs every test in its own thread).
    fn redact_id() -> Redaction {
        fn redacted_id(
            struct_name: &'static str,
            original_id: &str,
            map: &'static LocalKey<RefCell<HashMap<String, u64>>>,
            next_id: &'static LocalKey<RefCell<u64>>,
        ) -> Content {
            let id = map.with_borrow_mut(|map| {
                *map.entry(original_id.to_owned()).or_insert_with(|| {
                    next_id.with_borrow_mut(|next_id| {
                        let id = *next_id;
                        *next_id += 1;
                        id
                    })
                })
            });
            Content::NewtypeStruct(struct_name, Box::new(Content::U64(id)))
        }

        fn redacted_span_id(original_id: &str) -> Content {
            thread_local! {
                static SPAN_ID_TO_SEQUENTIAL_ID: RefCell<HashMap<String, u64>> = <_>::default();
                static NEXT_ID: RefCell<u64> = const { RefCell::new(1) };
            }
            redacted_id("SpanId", original_id, &SPAN_ID_TO_SEQUENTIAL_ID, &NEXT_ID)
        }

        fn redacted_request_id(original_id: &str) -> Content {
            thread_local! {
                static REQUEST_ID_TO_SEQUENTIAL_ID: RefCell<HashMap<String, u64>> = <_>::default();
                static NEXT_ID: RefCell<u64> = const { RefCell::new(1) };
            }
            redacted_id("RequestId", original_id, &REQUEST_ID_TO_SEQUENTIAL_ID, &NEXT_ID)
        }

        fn redact_recursive(value: Content) -> Content {
            match value {
                Content::NewtypeStruct(name @ ("SpanId" | "RequestId"), ref nested) => match **nested {
                    Content::String(ref original_id) => match name {
                        "SpanId" => redacted_span_id(original_id),
                        "RequestId" => redacted_request_id(original_id),
                        _ => unreachable!(),
                    },
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
            let _guard = info_span!(
                "test_span",
                request_id = RequestId::next().into_u64(),
                otel.kind = "client"
            )
            .entered();
        });

        let spans = collector.spans();

        assert_ron_snapshot!(
            spans,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          RequestId(1): [
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
    fn test_dynamic_request_id() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let span = info_span!("test_span", request_id = tracing::field::Empty);
            span.record("request_id", RequestId::next().into_u64());
            let _guard = span.enter();
        });

        let spans = collector.spans();

        assert_ron_snapshot!(
            spans,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          RequestId(1): [
            CollectedSpan(
              id: SpanId(1),
              parent_id: None,
              name: "test_span",
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
    fn test_nested_spans() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let _parent_guard = info_span!("parent_span", request_id = RequestId::next().into_u64()).entered();

            {
                let _child_guard = info_span!("child_span").entered();
                let _grandchild_guard = info_span!("grandchild_span").entered();
            }
        });

        let spans = collector.spans();

        assert_ron_snapshot!(
            spans,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          RequestId(1): [
            CollectedSpan(
              id: SpanId(1),
              parent_id: Some(SpanId(2)),
              name: "grandchild_span",
              attributes: {},
              kind: internal,
              links: [],
            ),
            CollectedSpan(
              id: SpanId(2),
              parent_id: Some(SpanId(3)),
              name: "child_span",
              attributes: {},
              kind: internal,
              links: [],
            ),
            CollectedSpan(
              id: SpanId(3),
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
            let _guard = info_span!(
                "attribute_span",
                request_id = RequestId::next().into_u64(),
                otel.kind = "client",
                string_attr = "value",
                bool_attr = true,
                int_attr = 42,
                float_attr = 3.5
            )
            .entered();
        });

        let spans = collector.spans();

        assert_ron_snapshot!(
            spans,
            {
                ".*" => redact_id(),
                ".*[].**" => redact_id(),
                ".*[].attributes" => insta::sorted_redaction()
            },
            @r#"
        {
          RequestId(1): [
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
            let span = info_span!(
                "updated_span",
                request_id = RequestId::next().into_u64(),
                otel.kind = "client",
                dynamic_attr = tracing::field::Empty
            );
            span.record("dynamic_attr", "added later");
            let _guard = span.enter();
        });

        let spans = collector.spans();

        assert_ron_snapshot!(
            spans,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          RequestId(1): [
            CollectedSpan(
              id: SpanId(1),
              parent_id: None,
              name: "updated_span",
              attributes: {
                "dynamic_attr": "added later",
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
    fn test_renamed_span() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let _guard = info_span!(
                "renamed_span",
                request_id = RequestId::next().into_u64(),
                otel.name = "new_name"
            )
            .entered();
        });

        let spans = collector.spans();

        assert_ron_snapshot!(
            spans,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          RequestId(1): [
            CollectedSpan(
              id: SpanId(1),
              parent_id: None,
              name: "new_name",
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
    fn test_follows_from() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let _scope = info_span!("parent_span", request_id = RequestId::next().into_u64()).entered();
            let span1 = info_span!("span1");
            let span2 = info_span!("span2");
            span2.follows_from(span1.id());
        });

        let spans = collector.spans();

        assert_ron_snapshot!(
            spans,
            {
                ".*" => redact_id(),
                ".*[].**" => redact_id(),
                ".*[].links[]" => redact_id(),
            },
            @r#"
        {
          RequestId(1): [
            CollectedSpan(
              id: SpanId(1),
              parent_id: Some(SpanId(2)),
              name: "span2",
              attributes: {},
              kind: internal,
              links: [
                SpanId(3),
              ],
            ),
            CollectedSpan(
              id: SpanId(3),
              parent_id: Some(SpanId(2)),
              name: "span1",
              attributes: {},
              kind: internal,
              links: [],
            ),
            CollectedSpan(
              id: SpanId(2),
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
    fn test_basic_event() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let _guard = info_span!("test_span", request_id = RequestId::next().into_u64()).entered();
            tracing::info!(name: "event", "test event");
        });

        let events = collector.events();

        assert_ron_snapshot!(
            events,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          RequestId(1): [
            CollectedEvent(
              span_id: SpanId(1),
              name: "event",
              level: Info,
              attributes: {
                "message": "test event",
              },
            ),
          ],
        }
        "#
        );
    }

    #[test]
    fn test_event_with_attributes() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let _guard = info_span!("test_span", request_id = RequestId::next().into_u64()).entered();

            tracing::info!(
                name: "event",
                string_attr = "value",
                bool_attr = true,
                int_attr = 42,
                float_attr = 3.5,
                "test event",
            );
        });

        let events = collector.events();

        assert_ron_snapshot!(
            events,
            {
                ".*" => redact_id(),
                ".*[].**" => redact_id(),
                ".*[].attributes" => insta::sorted_redaction()
            },
            @r#"
        {
          RequestId(1): [
            CollectedEvent(
              span_id: SpanId(1),
              name: "event",
              level: Info,
              attributes: {
                "bool_attr": true,
                "float_attr": 3.5,
                "int_attr": 42,
                "message": "test event",
                "string_attr": "value",
              },
            ),
          ],
        }
        "#
        );
    }

    #[test]
    fn test_events_in_nested_spans() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let _parent_guard = info_span!("parent_span", request_id = RequestId::next().into_u64()).entered();
            tracing::info!(name: "event1", "parent event");

            {
                let child = info_span!("child_span");
                let _child_guard = child.enter();
                tracing::info!(name: "event2", "child event");
            }
        });

        let events = collector.events();

        assert_ron_snapshot!(
            events,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          RequestId(1): [
            CollectedEvent(
              span_id: SpanId(1),
              name: "event1",
              level: Info,
              attributes: {
                "message": "parent event",
              },
            ),
            CollectedEvent(
              span_id: SpanId(2),
              name: "event2",
              level: Info,
              attributes: {
                "message": "child event",
              },
            ),
          ],
        }
        "#
        );
    }

    #[test]
    fn test_event_levels() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let _guard = info_span!("test_span", request_id = RequestId::next().into_u64()).entered();

            tracing::error!(name: "event1", "error event");
            tracing::warn!(name: "event2", "warn event");
            tracing::info!(name: "event3", "info event");
            tracing::debug!(name: "event4", "debug event");
            tracing::trace!(name: "event5", "trace event");

            tracing::info!(name: "event6", item_type = "query", "query event");
        });

        let events = collector.events();

        assert_ron_snapshot!(
            events,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          RequestId(1): [
            CollectedEvent(
              span_id: SpanId(1),
              name: "event1",
              level: Error,
              attributes: {
                "message": "error event",
              },
            ),
            CollectedEvent(
              span_id: SpanId(1),
              name: "event2",
              level: Warn,
              attributes: {
                "message": "warn event",
              },
            ),
            CollectedEvent(
              span_id: SpanId(1),
              name: "event3",
              level: Info,
              attributes: {
                "message": "info event",
              },
            ),
            CollectedEvent(
              span_id: SpanId(1),
              name: "event4",
              level: Debug,
              attributes: {
                "message": "debug event",
              },
            ),
            CollectedEvent(
              span_id: SpanId(1),
              name: "event5",
              level: Trace,
              attributes: {
                "message": "trace event",
              },
            ),
            CollectedEvent(
              span_id: SpanId(1),
              name: "event6",
              level: Query,
              attributes: {
                "message": "query event",
              },
            ),
          ],
        }
        "#
        );
    }

    #[test]
    fn test_ignored_attributes() {
        let collector = TestCollector::new();
        let subscriber = Registry::default().with(layer(collector.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let _guard = info_span!(
                "test_span",
                request_id = RequestId::next().into_u64(),
                test_ignored_span_attr = "ignored",
                kept_attr = "kept"
            )
            .entered();

            tracing::info!(
                name: "event",
                test_ignored_event_attr = "ignored",
                kept_attr = "kept",
                "test event"
            );
        });

        let spans = collector.spans();
        let events = collector.events();

        assert_ron_snapshot!(
            spans,
            { ".*" => redact_id(), ".*[].**" => redact_id() },
            @r#"
        {
          RequestId(1): [
            CollectedSpan(
              id: SpanId(1),
              parent_id: None,
              name: "test_span",
              attributes: {
                "kept_attr": "kept",
              },
              kind: internal,
              links: [],
            ),
          ],
        }
        "#
        );

        assert_ron_snapshot!(
            events,
            {
                ".*" => redact_id(),
                ".*[].**" => redact_id(),
                ".*[].attributes" => insta::sorted_redaction()
            },
            @r#"
        {
          RequestId(1): [
            CollectedEvent(
              span_id: SpanId(1),
              name: "event",
              level: Info,
              attributes: {
                "kept_attr": "kept",
                "message": "test event",
              },
            ),
          ],
        }
        "#
        );
    }
}
