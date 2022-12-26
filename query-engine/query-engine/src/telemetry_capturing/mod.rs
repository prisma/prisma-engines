pub use self::capturer::Capturer;
use self::capturer::Exporter;
use self::capturer::SyncedSpanProcessor;
use once_cell::sync::Lazy;
use opentelemetry::{global, sdk, trace};

static CAPTURER: Lazy<capturer::Exporter> = Lazy::new(Exporter::default);
static PROCESSOR: Lazy<SyncedSpanProcessor> = Lazy::new(|| SyncedSpanProcessor::new(CAPTURER.to_owned()));
static TRACER: Lazy<sdk::trace::Tracer> = Lazy::new(setup_and_install_tracer_globally);

// Creates a new capturer, which is configured to export traces and log events happening during a
// particular request
pub fn capturer(trace_id: trace::TraceId, settings: &str) -> Capturer {
    Capturer::new(CAPTURER.to_owned(), trace_id, settings.into())
}

// Reference to the global processor used by the tracer and used for deterministic flushing
// of the spans that are pending to be processed after a request finishes.
pub(self) fn global_processor() -> SyncedSpanProcessor {
    PROCESSOR.to_owned()
}

// Reference to the global tracer used when capturing telemetry in the response
pub fn global_tracer() -> &'static sdk::trace::Tracer {
    &TRACER
}

// Installs an opentelemetry tracer globally, which is configured to proecss
// spans and export them to global exporter.
fn setup_and_install_tracer_globally() -> sdk::trace::Tracer {
    global::set_text_map_propagator(sdk::propagation::TraceContextPropagator::new());

    let provider_builder = sdk::trace::TracerProvider::builder().with_span_processor(global_processor());
    let provider = provider_builder.build();
    let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, "opentelemetry");

    global::set_tracer_provider(provider);
    tracer
}

pub mod capturer;
pub mod models;
mod settings;
pub mod storage;
