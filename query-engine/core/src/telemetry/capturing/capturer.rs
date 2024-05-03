use super::{settings::Settings, storage::Storage};
use crate::models;
use opentelemetry::{
    sdk::{
        export::trace::SpanData,
        trace::{Span, SpanProcessor},
    },
    trace::{TraceId, TraceResult},
};

/// Capturer determines, based on a set of settings and a trace id, how capturing is going to be handled.
/// Generally, both the trace id and the settings will be derived from request headers. Thus, a new
/// value of this enum is created per request.
#[derive(Debug, Clone)]
pub enum Capturer {
    Enabled(Inner),
    Disabled,
}

impl Capturer {
    pub(super) fn new(processor: Processor, trace_id: TraceId, settings: Settings) -> Self {
        if settings.is_enabled() {
            return Self::Enabled(Inner {
                processor,
                trace_id,
                settings,
            });
        }

        Self::Disabled
    }
}

#[derive(Debug, Clone)]
pub struct Inner {
    pub(super) processor: Processor,
    pub(super) trace_id: TraceId,
    pub(super) settings: Settings,
}

impl Inner {
    pub async fn start_capturing(&self) {
        self.processor
            .start_capturing(self.trace_id, self.settings.clone())
            .await
    }

    pub async fn fetch_captures(&self) -> Option<Storage> {
        self.processor.fetch_captures(self.trace_id).await
    }
}

/// A [`SpanProcessor`] that captures and stores spans in memory in a synchronized dictionary for
/// later retrieval
#[derive(Debug, Clone)]
pub struct Processor {}

impl Processor {
    pub fn new() -> Self {
        Self {}
    }

    async fn start_capturing(&self, trace_id: TraceId, settings: Settings) {
        task::start_capturing(trace_id, settings).await.unwrap();
    }

    async fn fetch_captures(&self, trace_id: TraceId) -> Option<Storage> {
        task::fetch_captures_for_trace(trace_id).await.ok()
    }
}

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}

impl SpanProcessor for Processor {
    fn on_start(&self, _: &mut Span, _: &opentelemetry::Context) {
        // no-op
    }

    /// Exports a span containing zero or more events that might represent
    /// logs in Prisma Client logging categories of logs (query, info, warn, error)
    ///
    /// There's an impedance between the client categories of logs and the server (standard)
    /// hierarchical levels of logs (trace, debug, info, warn, error).
    ///
    /// The most prominent difference is the "query" type of events. In the client these model
    /// database queries made by the engine through a connector. But ATM there's not a 1:1 mapping
    /// between the client "query" level and one of the server levels. And depending on the database
    /// mongo / relational, the information to build this kind of log event is logged diffeerently in
    /// the server.
    ///
    /// In the case of the of relational databaes --queried through sql_query_connector and eventually
    /// through quaint, a trace span describes the query-- `TraceSpan::represents_query_event`
    /// determines if a span represents a query event.
    ///
    /// In the case of mongo, an event represents the query, but it needs to be transformed before
    /// capturing it. `Event::query_event` does that.
    fn on_end(&self, span_data: SpanData) {
        task::span_data_processed(span_data).unwrap();
    }

    fn force_flush(&self) -> TraceResult<()> {
        // no-op
        Ok(())
    }

    fn shutdown(&mut self) -> TraceResult<()> {
        // no-op
        Ok(())
    }
}

mod task {
    use super::*;
    use crossbeam_channel::*;
    use once_cell::sync::Lazy;
    use std::collections::HashMap;
    use tokio::sync::oneshot;

    const VALID_QUERY_ATTRS: [&str; 4] = ["query", "params", "target", "duration_ms"];
    /// A Candidate represents either a span or an event that is being considered for capturing.
    /// A Candidate can be converted into a [`Capture`].
    #[derive(Debug, Clone)]
    struct Candidate<'batch_iter> {
        value: models::LogEvent,
        settings: &'batch_iter Settings,
    }

    impl Candidate<'_> {
        fn is_loggable_query_event(&self) -> bool {
            if self.settings.included_log_levels.contains("query") {
                if let Some(target) = self.value.attributes.get("target") {
                    if let Some(val) = target.as_str() {
                        return (val == "quaint::connector::metrics" && self.value.attributes.contains_key("query"))
                            || val == "mongodb_query_connector::query";
                    }
                }
            }
            false
        }

        fn query_event(mut self) -> models::LogEvent {
            self.value
                .attributes
                .retain(|key, _| VALID_QUERY_ATTRS.contains(&key.as_str()));

            models::LogEvent {
                level: "query".to_string(),
                ..self.value
            }
        }

        fn is_loggable_event(&self) -> bool {
            self.settings.included_log_levels.contains(&self.value.level)
        }
    }

    #[derive(Debug)]
    pub(super) enum CaptureOp {
        /// Tells the task that the given span data has been processed by the span processor
        SpanDataProcessed(SpanData),
        /// Tells the task to start capturing for the given trace id
        StartCapturing(TraceId, Settings, oneshot::Sender<()>),
        /// Tells the task to fetch the captures for the given trace_id, and sendthem to the given sender
        FetchCaptures(TraceId, oneshot::Sender<Storage>),
    }

    static SENDER: Lazy<Sender<CaptureOp>> = Lazy::new(|| {
        let (sender, receiver) = unbounded();

        std::thread::spawn(move || {
            let mut store: HashMap<TraceId, Storage> = Default::default();

            loop {
                match receiver.recv() {
                    Ok(CaptureOp::StartCapturing(trace_id, settings, op_sender)) => {
                        tracing::trace!("capture task: start capturing for {:?}", trace_id);

                        let storage = Storage::from(settings);
                        store.insert(trace_id, storage);
                        _ = op_sender.send(());
                    }
                    Ok(CaptureOp::SpanDataProcessed(span_data)) => {
                        tracing::trace!("capture task: sending span data {:?}", span_data);
                        let trace_id = span_data.span_context.trace_id();

                        if let Some(storage) = store.get_mut(&trace_id) {
                            let settings = storage.settings.clone();
                            let (events, span) = models::TraceSpan::from(span_data).split_events();

                            if settings.traces_enabled() {
                                storage.traces.push(span);
                            }

                            if storage.settings.logs_enabled() {
                                events.into_iter().for_each(|log| {
                                    let candidate = Candidate {
                                        value: log,
                                        settings: &settings,
                                    };
                                    if candidate.is_loggable_query_event() {
                                        storage.logs.push(candidate.query_event())
                                    } else if candidate.is_loggable_event() {
                                        storage.logs.push(candidate.value)
                                    }
                                });
                            }
                        }
                    }
                    Ok(CaptureOp::FetchCaptures(trace_id, sender)) => {
                        tracing::info!("fetching captures for trace_id={:?}.", trace_id);
                        if let Some(storage) = store.remove(&trace_id) {
                            match sender.send(storage) {
                                Ok(_) => (),
                                Err(_) => {
                                    tracing::error!(
                                        "send error in capture task, when fetching captures for trace_id={:?}.",
                                        trace_id
                                    );
                                }
                            }
                        } else {
                            tracing::error!("storage should contain captures for (trace_id={:?})", trace_id);
                            _ = sender.send(Storage::default());
                        }
                    }
                    Err(_) => {
                        tracing::error!("recv error in capture task");
                        unreachable!("CAPTURE_TASK channel closed")
                    }
                }
            }
        });

        sender
    });

    pub(super) fn span_data_processed(span_data: SpanData) -> Result<(), SendError<CaptureOp>> {
        SENDER.send(CaptureOp::SpanDataProcessed(span_data))
    }

    pub(crate) async fn start_capturing(
        trace_id: TraceId,
        settings: Settings,
    ) -> Result<(), tokio::sync::oneshot::error::RecvError> {
        let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
        SENDER
            .send(CaptureOp::StartCapturing(trace_id, settings, sender))
            .unwrap();
        receiver.await
    }

    pub(crate) async fn fetch_captures_for_trace(
        trace_id: TraceId,
    ) -> Result<Storage, tokio::sync::oneshot::error::RecvError> {
        let (sender, receiver) = oneshot::channel::<Storage>();
        SENDER.send(CaptureOp::FetchCaptures(trace_id, sender)).unwrap();
        receiver.await
    }
}
