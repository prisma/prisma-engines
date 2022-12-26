use std::time::Duration;

use once_cell::sync::Lazy;
use opentelemetry::{global, sdk, trace};

use self::capturer::{Capturer, Exporter};

pub mod capturer;
pub mod models;
mod settings;
pub mod storage;

static CAPTURER: Lazy<capturer::Exporter> = Lazy::new(Exporter::default);
static TRACER: Lazy<sdk::trace::Tracer> = Lazy::new(setup_and_install_tracer_globally);

// Returns a reference to the global tracer used when capturing telemetry in the response
pub fn tracer() -> &'static sdk::trace::Tracer {
    &TRACER
}

// Creates a new capturer, which is configured to export traces and log events happening during a
// particular request
pub fn capturer(trace_id: trace::TraceId, settings: &str) -> Capturer {
    Capturer::new(CAPTURER.to_owned(), trace_id, settings.into())
}

// Installs an opentelemetry tracer globally, which is configured to proecss
// spans and export them to global exporter.
fn setup_and_install_tracer_globally() -> sdk::trace::Tracer {
    global::set_text_map_propagator(sdk::propagation::TraceContextPropagator::new());

    let processor = sdk::trace::BatchSpanProcessor::builder(CAPTURER.to_owned(), opentelemetry::runtime::Tokio)
        .with_scheduled_delay(Duration::new(0, 1))
        .build();

    let provider_builder = sdk::trace::TracerProvider::builder().with_span_processor(processor);

    let provider = provider_builder.build();
    let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, "opentelemetry");
    global::set_tracer_provider(provider);

    tracer
}
