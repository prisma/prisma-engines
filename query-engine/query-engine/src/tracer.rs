use async_trait::async_trait;
use once_cell::sync::{Lazy, OnceCell};
use opentelemetry::{
    global,
    sdk::{self, export::ExportError},
    sdk::{
        export::trace::{ExportResult, SpanData, SpanExporter},
        propagation::TraceContextPropagator,
    },
    trace::{TraceId, TracerProvider},
};
use query_core::telemetry;
use std::io::{stdout, Stdout};
use std::{fmt::Debug, io::Write};

static TRACER: OnceCell<opentelemetry::sdk::trace::Tracer> = OnceCell::new();

pub(crate) fn tracer() -> &'static opentelemetry::sdk::trace::Tracer {
    TRACER.get().unwrap()
}

pub(crate) fn install<E>(exporter: Option<E>, mut tracer_config: Option<sdk::trace::Config>)
where
    E: SpanExporter + 'static,
{
    global::set_text_map_propagator(TraceContextPropagator::new());
    let mut provider_builder = sdk::trace::TracerProvider::builder();

    if let Some(exporter) = exporter {
        provider_builder = provider_builder.with_simple_exporter(exporter);
    }

    if let Some(config) = tracer_config.take() {
        provider_builder = provider_builder.with_config(config);
    }
    let provider = provider_builder.build();
    TRACER
        .set(provider.tracer("opentelemetry"))
        .expect("tracer::install() was called twice. This is a bug.");
    global::set_tracer_provider(provider);
}

/// A [`ClientSpanExporter`] that sends spans to stdout.
#[derive(Debug)]
pub struct ClientSpanExporter {
    writer: Stdout,
}

impl ClientSpanExporter {
    pub fn new() -> Self {
        Self { writer: stdout() }
    }
}

impl Default for ClientSpanExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SpanExporter for ClientSpanExporter {
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        let result = telemetry::helpers::spans_to_json(batch);

        if let Err(err) = writeln!(self.writer, "{result}") {
            Err(ClientSpanExporterError(err).into())
        } else {
            Ok(())
        }
    }
}

// ClientSpanExporter exporter's error
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
struct ClientSpanExporterError(std::io::Error);

impl ExportError for ClientSpanExporterError {
    fn exporter_name(&self) -> &'static str {
        "stdout"
    }
}

#[derive(Debug)]
pub struct CapturingExporter;

#[async_trait::async_trait]
impl SpanExporter for CapturingExporter {
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        for span in batch {
            // todo: own error type to avoid the unwrap here
            capture_task::send_span(span.span_context.trace_id(), span)
                .await
                .unwrap();
        }

        Ok(())
    }
}

mod capture_task {
    use super::*;
    use crossbeam_channel::*;
    use std::collections::HashMap;

    #[derive(Debug)]
    pub(super) enum CaptureTaskRequest {
        Send(TraceId, SpanData),
        FetchCaptures(TraceId, tokio::sync::oneshot::Sender<String>),
    }

    struct Settings;

    static CAPTURE_TASK: Lazy<Sender<CaptureTaskRequest>> = Lazy::new(|| {
        let (sender, receiver) = unbounded();

        std::thread::spawn(move || {
            let mut store: HashMap<TraceId, (Settings, String)> = Default::default();

            loop {
                match receiver.recv() {
                    Ok(CaptureTaskRequest::Send(trace_id, span_data)) => {
                        tracing::info!("receiving trace for {:?}", trace_id);
                        let traces = store.entry(trace_id).or_insert_with(|| (Settings, String::new()));
                        traces.1.push_str(&format!("{:#?}", span_data));
                    }
                    Ok(CaptureTaskRequest::FetchCaptures(trace_id, send)) => {
                        tracing::info!("fetching captures for {:?}", trace_id);
                        if let Some(traces) = store.remove(&trace_id) {
                            match send.send(traces.1) {
                                Ok(_) => (),
                                Err(_) => {
                                    tracing::error!("here2");
                                }
                            }
                        } else {
                            tracing::error!("here");
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

    // Missing concern: filtering

    pub(super) async fn send_span(trace_id: TraceId, span: SpanData) -> Result<(), ()> {
        CAPTURE_TASK
            .send(CaptureTaskRequest::Send(trace_id, span))
            .map_err(|_| ())
    }

    pub(crate) async fn fetch_captures_for_trace(trace_id: TraceId) -> Result<String, Box<dyn std::error::Error>> {
        let (sender, mut receiver) = tokio::sync::oneshot::channel::<String>();
        CAPTURE_TASK
            .send(CaptureTaskRequest::FetchCaptures(trace_id, sender))
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(receiver.try_recv().unwrap())
    }
}

pub(crate) use capture_task::fetch_captures_for_trace;
