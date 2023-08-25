use async_trait::async_trait;
use napi::{threadsafe_function::ThreadsafeFunctionCallMode, Status};
use opentelemetry::{
    global, sdk,
    sdk::{
        export::trace::{ExportResult, SpanData, SpanExporter},
        propagation::TraceContextPropagator,
    },
    trace::{TraceError, TracerProvider},
};
use query_core::telemetry;
use std::fmt::{self, Debug};

use crate::logger::LogCallback;

/// Pipeline builder
#[derive(Debug)]
pub struct PipelineBuilder {
    trace_config: Option<sdk::trace::Config>,
}

/// Create a new stdout exporter pipeline builder.
pub fn new_pipeline() -> PipelineBuilder {
    PipelineBuilder::default()
}

impl Default for PipelineBuilder {
    /// Return the default pipeline builder.
    fn default() -> Self {
        Self { trace_config: None }
    }
}

impl PipelineBuilder {
    /// Assign the SDK trace configuration.
    #[allow(dead_code)]
    pub fn with_trace_config(mut self, config: sdk::trace::Config) -> Self {
        self.trace_config = Some(config);
        self
    }
}

impl PipelineBuilder {
    pub fn install_simple(mut self, log_callback: LogCallback) -> sdk::trace::Tracer {
        global::set_text_map_propagator(TraceContextPropagator::new());
        let exporter = ClientSpanExporter::new(log_callback);

        let mut provider_builder = sdk::trace::TracerProvider::builder().with_simple_exporter(exporter);
        // This doesn't work at the moment because we create the logger outside of an async runtime
        // we could later move the creation of logger into the `connect` function
        // let mut provider_builder = sdk::trace::TracerProvider::builder().with_batch_exporter(exporter, runtime::Tokio);
        // remember to add features = ["rt-tokio"] to the cargo.toml
        if let Some(config) = self.trace_config.take() {
            provider_builder = provider_builder.with_config(config);
        }
        let provider = provider_builder.build();
        let tracer = provider.tracer("opentelemetry");
        global::set_tracer_provider(provider);

        tracer
    }
}

/// A [`ClientSpanExporter`] that sends spans to the JS callback.
pub struct ClientSpanExporter {
    callback: LogCallback,
}

impl ClientSpanExporter {
    pub fn new(callback: LogCallback) -> Self {
        Self { callback }
    }
}

impl Debug for ClientSpanExporter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientSpanExporter").finish()
    }
}

#[async_trait]
impl SpanExporter for ClientSpanExporter {
    /// Export spans to stdout
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        let result = telemetry::helpers::spans_to_json(batch);
        let status = self.callback.call(result, ThreadsafeFunctionCallMode::Blocking);

        if status != Status::Ok {
            return Err(TraceError::from(format!("Could not call JS callback: {}", status)));
        }

        Ok(())
        // .map_err(|err| T))
    }
}
