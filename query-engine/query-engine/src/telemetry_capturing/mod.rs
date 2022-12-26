//! Telemetry Capturing is the process of recording the logs and traces happening during a request
//! to the binary engine, and rendering them in the response.
//!
//! The interaction diagram below (soorry width!) shows the different roles at play during telemetry
//! capturing. A textual explanatation follows it.
//!
//!                                                             ╔═══════════════════════╗ ╔═══════════════╗ ╔═══════════════════════╗                                                           
//!                                                             ║<<SpanExporter, Sync>> ║ ║    Storage    ║ ║<<SpanProcessor, Sync>>║ ╔═══════════════════╗                                     
//!        ┌───────────────────┐                                ║       EXPORTER        ║ ║               ║ ║       PROCESSOR       ║ ║      TRACER       ║                                     
//!        │      Server       │                                ╚═══════════╦═══════════╝ ╚═══════╦═══════╝ ╚═══════════╦═══════════╝ ╚═════════╦═════════╝                                     
//!        └─────────┬─────────┘                                            │                     │                     │                       │                                               
//!                  │                                                      │                     │                     │                       │                                               
//!                  │                                                      │                     │                     │                       │                                               
//!          POST    │                                                      │                     │                     │                       │                                               
//!     (body, header)                                                      │                     │                     │                       │                                               
//!      ──────────▶┌┴┐                                                     │                     │                     │                       │                                               
//!                 │ │new(headers)╔════════════╗                           │                     │                     │                       │                                               
//!                 │ ├───────────▶║s: Settings ║                           │                     │                     │                       │                                               
//!                 │ │            ╚════════════╝                           │                     │                     │                       │                                               
//!                 │ │                                                     │                     │                     │                       │                                               
//!                 │ │                    ╔═══════════════════╗            │                     │                     │                       │                                               
//!                 │ │                    ║ Capturer::Enabled ║            │                     │                     │                       │             ┌────────────┐                    
//!                 │ │                    ╚═══════════════════╝            │                     │                     │                       │             │<<Somewhere>│                    
//!                 │ │                              │                      │                     │                     │                       │             └──────┬─────┘                    
//!                 │ │        new(trace_id, s)      │                      │                     │                     │                       │                    │                          
//!                 │ ├─────────────────────────────▶│                      │                     │                     │                       │                    │                          
//!                 │ │                              │                      │                     │                     │                       │                    │                          
//!                 │ │                              │                      │                     │                     │                       │                    │                          
//!                 │ │       start_capturing()      │                      │                     │                     │                       │                    │                          
//!                 │ ├─────────────────────────────▶│                      │                     │                     │                       │                    │                          
//!                 │ │                              │                      │                     │                     │                       │                    │                          
//!                 │ │                              │    start_capturing   │                     │                     │                       │                    │                          
//!                 │ │                              │     (trace_id, s)    │                     │                     │                       │                    │                          
//!                 │ │                              ├──────────────────────┼─insert(trace_id, s) │                     │                       │                    │                          
//!                 │ │                              │                      ├────────────────────▶│                     │                       │                    │                          
//!                 │ │                              │                      │                     │                     │                       │                    │                          
//!                 │ │                              │                      │                     │                     │                       │ process_query      │                          
//!                 │ │──────────────────────────────┼──────────────────────┼─────────────────────┼─────────────────────┼───────────────────────┼───────────────────┌┴┐                         
//!                 │ │                              │                      │                     │                     │                       │                   │ │                         
//!                 │ │                              │                      │                     │                     │                       │                   │ │                         
//!                 │ │                              │                      │                     │                     │                       │      log! / span! │ │  ┌─────────────────────┐
//!                 │ │                              │                      │                     │                     │   on_start / on_end   ◀───────────────────│ │  │ res: PrismaResponse │
//!                 │ │                              │                      │                     │                     │◀──────────────────────┤                   │ │  └──────────┬──────────┘
//!                 │ │                              │                      │      export(Vec<Span>)                    │                       │                   │ │   new       │           
//!                 │ │                              │                      │◀────────────────────┼─────────────────────│                       │                   │ │────────────▶│           
//!                 │ │                              │                      │                     │                     │                       │                   │ │             │           
//!                 │ │                              │                      │   append(trace_id,  │                     │                       │                   │ │             │           
//!                 │ │                              │                      │     logs, traces)   │                     │                       │                   │ │             │           
//!                 │ │                              │                      ├────────────────────▶│                     │                       │                   │ │             │           
//!                 │ │                              │                      │                     │                     │                       │                   │ │             │           
//!                 │ │      res: PrismaResponse     │                      │                     │                     │                       │                   │ │             │           
//!                 │ │◁ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┼ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─└┬┘             │           
//!                 │ │                              │                      │                     │                     │                       │                    │              │           
//!                 │ │                              │  Flush()             │                     │                     │                       │                    │              │           
//!                 │ ├──────────────────────────────┼──────────────────────┼────────────────────▶│                     │                       │                    │              │           
//!                 │ │                              │                      │                     │                     │                       │                    x              │           
//!                 │ │        fetch_captures()      │                      │                     │                     │                       │                                   │           
//!                 │ ├─────────────────────────────▶│      fetch_captures  │                     │                     │                       │                                   │           
//!                 │ │                              │        (trace_id)    │                     │                     │                       │                                   │           
//!                 │ │                              ├─────────────────────▶│                     │                     │                       │                                   │           
//!                 │ │                              │                      │                     │                     │                       │                                   │           
//!                 │ │                              │                      │                     │                     │                       │                                   │           
//!                 │ │                              ◁ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│                     │                     │                       │                                   │           
//!                 │ │◁─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│                      │                     │                     │                       │                                   │           
//!                 │ │          logs, traces        x                      │                     │                     │                       │                                   │           
//!                 │ │                                                     │                     │                     │                       │        res.set_extension(logs)    │           
//!                 │ ├─────────────────────────────────────────────────────┼─────────────────────┼─────────────────────┼───────────────────────┼──────────────────────────────────▶│           
//!                 │ │                                                     │                     │                     │                       │        res.set_extension(traces)  │           
//!                 │ ├─────────────────────────────────────────────────────┼─────────────────────┼─────────────────────┼───────────────────────┼──────────────────────────────────▶│           
//!                 │ │                                                     │                     │                     │                       │                                   x           
//!          ◀ ─ ─ ─└┬┘                                                                                                                                                                         
//!         json!(res)                                                                                                                                                                          
//!                                                                                                                                                                                             
//!                                                                                                                                                                                             
//!                                                                                                                                                                                             
//!                                                                            ◁─ ─ ─ ─ return                                                                                                  
//!                                                                                                                                                                                             
//!                                                                            ───────▶ function invocation  (pseudo-signature)                                                                                  
//!                                                                                                                                                                                            
//!
//!  In the diagram you will see objects whose lifetime is static. The boxes for those have a double
//! width margin. These are:
//!
//!   - The `server` itself
//!   - The  global `TRACER`, which handles `log!` and `span!` and uses the global `PROCESSOR` to
//!     process the data constituting a trace `Span`s and log `Event`s
//!   - The global `EXPORTER`, which manages the `Storage` set of datastructures, holding logs,
//!     traces (and capture settings) per request.
//!
//! Then throughout the request lifecycle, different objects are created and dropped:
//!
//!   - When a request comes in, its headers are processed and a [`Settings`] object is built, this
//!     object determines, for the request, how logging and tracing is going to be captured: if only
//!     traces, or logs, or both, and which log levels are going to be captured.
//!   - Based on the settings, a new `Capturer` is created, a capturer is nothing but a exporter
//!     wrapped to start capturing / fetch the captures for this particular request.
//!
//! Then the capturing proccess works in this way:
//!    
//!   1. The server receives a query
//!   2. It grabs the headers, and builds a `Capture` object, which is configured with the settings
//!      denoted by the `X-capture-telemetry`
//!   3. Now the server tells the `Capturer` to start capturing all the logs and traces occurring on
//!     the request (denoted by a `trace_id`) The `trace_id` is either carried on the `traceparent`
//!     header or implicitly created on the first span of the request. To _start capturing_ implies
//!     creating for the `trace_id` in two different datastructures: `logs` and `traces`; and storing
//!     the to be used for selecting the Spans and Event to capture.
//!   4. The server dispatches the request and anywhere else in the code, it is processed. There
//!     the code logs events and emits traces asynchronously.
//!   5. Traces and Logs arrive at the `TRACER`, which get built using the `PROCESSOR`, and exported
//!     in batches by the `EXPORTER` which writes them in the shard corresponding to the current
//!     `trace_id`, into the `logs` and `traces` storage. The settings previously stored for the
//!     `trace_id` are used to pick which events and spans are going to be captured based on their
//!     level.
//!   6. When the code that dispatches the request is done (represented by `<<Somewhere>>` in the
//!     diagram) it returns a `PrismaResponse` to the server.
//!   7. Then the server flushes the `PROCESSOR` to export any pending `Span`s and `Event`s.
//!   8. And right after that, it fetches the captures from the `EXPORTER`s `Storage`. At that time,
//!     altough that's not represented in the diagram, the captures are deleted from the storage,
//!     thus freeing any memory used for capturing during the request
//!   9. Finally the server sets the `logs` and `traces`extensions in the `PrismaResponse`, serializes
//!     it in json format and returns it back in as an HTTP Response blob.
//!                
pub use self::capturer::Capturer;
use self::capturer::Exporter;
use self::capturer::SyncedSpanProcessor;
use once_cell::sync::Lazy;
use opentelemetry::{global, sdk, trace};

static EXPORTER: Lazy<capturer::Exporter> = Lazy::new(Exporter::default);
static PROCESSOR: Lazy<SyncedSpanProcessor> = Lazy::new(|| SyncedSpanProcessor::new(EXPORTER.to_owned()));
static TRACER: Lazy<sdk::trace::Tracer> = Lazy::new(setup_and_install_tracer_globally);

/// Creates a new capturer, which is configured to export traces and log events happening during a
/// particular request
pub fn capturer(trace_id: trace::TraceId, settings: &str) -> Capturer {
    Capturer::new(EXPORTER.to_owned(), trace_id, settings.into())
}

/// Returns a clone of the global processor used by the tracer and used for deterministic flushing
/// of the spans that are pending to be processed after a request finishes.
pub(self) fn global_processor() -> SyncedSpanProcessor {
    PROCESSOR.to_owned()
}

/// Returns a clone to the global tracer used when capturing telemetry in the response
pub fn global_tracer() -> &'static sdk::trace::Tracer {
    &TRACER
}

/// Installs an opentelemetry tracer globally, which is configured to proecss
/// spans and export them to global exporter.
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
