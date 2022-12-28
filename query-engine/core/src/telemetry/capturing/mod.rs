//! Telemetry Capturing is the process of recording the logs and traces happening during a request
//! to the binary engine, and rendering them in the response.
//!
//! The interaction diagram below (soorry width!) shows the different roles at play during telemetry
//! capturing. A textual explanatation follows it. For the sake of example a server environment
//! --the query-engine crate-- is assumed.
//!
//! ```
//!
//!                                                              ╔═══════════════════════╗ ╔═══════════════╗ ╔═══════════════════════╗                                                           
//!                                                              ║<<SpanExporter, Sync>> ║ ║    Storage    ║ ║<<SpanProcessor, Sync>>║ ╔═══════════════════╗                                     
//!         ┌───────────────────┐                                ║       EXPORTER        ║ ║               ║ ║       PROCESSOR       ║ ║      TRACER       ║                                     
//!         │      Server       │                                ╚═══════════╦═══════════╝ ╚═══════╦═══════╝ ╚═══════════╦═══════════╝ ╚═════════╦═════════╝                                     
//!         └─────────┬─────────┘                                            │                     │                     │                       │                                               
//!                   │                                                      │                     │                     │                       │                                               
//!                   │                                                      │                     │                     │                       │                                               
//!         POST      │                                                      │                     │                     │                       │                                               
//!    (body, headers)│                                                      │                     │                     │                       │                                               
//!       ──────────▶┌┴┐                                                     │                     │                     │                       │                                               
//!          [1]     │ │new(headers)╔════════════╗                           │                     │                     │                       │                                               
//!                  │ ├───────────▶║s: Settings ║                           │                     │                     │                       │                                               
//!                  │ │    [2]     ╚════════════╝                           │                     │                     │                       │                                               
//!                  │ │                                                     │                     │                     │                       │                                               
//!                  │ │                    ╔═══════════════════╗            │                     │                     │                       │                                               
//!                  │ │                    ║ Capturer::Enabled ║            │                     │                     │                       │             ┌────────────┐                    
//!                  │ │                    ╚═══════════════════╝            │                     │                     │                       │             │<<Somewhere>│                    
//!                  │ │                              │                      │                     │                     │                       │             └──────┬─────┘                    
//!                  │ │        new(trace_id, s)      │                      │                     │                     │                       │                    │                          
//!                  │ ├─────────────────────────────▶│                      │                     │                     │                       │                    │                          
//!                  │ │           [2]                │                      │                     │                     │                       │                    │                          
//!                  │ │                              │                      │                     │                     │                       │                    │                          
//!                  │ │       start_capturing()      │                      │                     │                     │                       │                    │                          
//!                  │ ├─────────────────────────────▶│                      │                     │                     │                       │                    │                          
//!                  │ │            [3]               │                      │                     │                     │                       │                    │                          
//!                  │ │                              │    start_capturing   │                     │                     │                       │                    │                          
//!                  │ │                              │     (trace_id, s)    │                     │                     │                       │                    │                          
//!                  │ │                              ├─────────────────────▶│ insert(trace_id, s) │                     │                       │                    │                          
//!                  │ │                              │                      ├────────────────────▶│                     │                       │                    │                          
//!                  │ │                              │                      │        [4]          │                     │                       │                    │                          
//!                  │ │                              │                      │                     │                     │                       │  process_query     │                          
//!                  │ │──────────────────────────────┼──────────────────────┼─────────────────────┼─────────────────────┼───────────────────────┼──────────────────▶┌┴┐                         
//!                  │ │                              │                      │                     │                     │                       │       [5]         │ │                         
//!                  │ │                              │                      │                     │                     │                       │                   │ │                         
//!                  │ │                              │                      │                     │                     │                       │     log! / span!  │ │  ┌─────────────────────┐
//!                  │ │                              │                      │                     │                     │   on_start / on_end   ◀───────────────────│ │  │ res: PrismaResponse │
//!                  │ │                              │                      │                     │                     │◀──────────────────────┤       [6]         │ │  └──────────┬──────────┘
//!                  │ │                              │                      │      export(Vec<Span>) [8]                │        [7]            │                   │ │   new       │           
//!                  │ │                              │                      │◀────────────────────┼─────────────────────│                       │                   │ │────────────▶│           
//!                  │ │                              │                      │                     │                     │                       │                   │ │             │           
//!                  │ │                              │                      │   append(trace_id,  │                     │                       │                   │ │             │           
//!                  │ │                              │                      │     logs, traces)   │                     │                       │                   │ │             │           
//!                  │ │                              │                      ├────────────────────▶│                     │                       │                   │ │             │           
//!                  │ │                              │                      │        [9]          │                     │                       │                   │ │             │           
//!                  │ │      res: PrismaResponse [10]│                      │                     │                     │                       │                   │ │             │           
//!                  │ │◁ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┼ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─└┬┘             │           
//!                  │ │        fetch_captures()      │                      │                     │                     │                       │                    │              │           
//!                  │ ├─────────────────────────────▶│      fetch_captures  │                     │                     │                       │                    │              │           
//!                  │ │             [11]             │        (trace_id)    │                     │                     │                       │                    │              │           
//!                  │ │                              ├─────────────────────▶│                     │   Flush()[12]       │                       │                    x              │           
//!                  │ │                              │                      ├─────────────────────┼────────────────────▶│                       │                                   │           
//!                  │ │                              │                      │                     │                     │                       │                                   │           
//!                  │ │                              │                      │        export(pending: Vec<Span>)         │                       │                                   │           
//!                  │ │                              │                      │◀────────────────────┼─────────────────────│                       │                                   │           
//!                  │ │                              │                      │                     │                     │                       │                                   │           
//!                  │ │                              │                    get logs/traces for trace_id                  │                       │                                   │           
//!                  │ │                              │                      ├────────────────────▶│                     │                       │                                   │           
//!                  │ │                              ◁ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│      [13]           │                     │                       │                                   │           
//!                  │ │◁─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│                      │                     │                     │                       │                                   │           
//!                  │ │          logs, traces        x                      │                     │                     │                       │        res.set_extension(logs)    │           
//!                  │ ├─────────────────────────────────────────────────────┼─────────────────────┼─────────────────────┼───────────────────────┼─[14]─────────────────────────────▶│           
//!                  │ │                                                     │                     │                     │                       │        res.set_extension(traces)  │           
//!                  │ ├─────────────────────────────────────────────────────┼─────────────────────┼─────────────────────┼───────────────────────┼──────────────────────────────────▶│           
//!           ◀ ─ ─ ─└┬┘                                                     │                     │                     │                       │                                   x           
//!        json!(res)                                                                                                                                                                           
//!          [15]                                                                                                                                                                                                                                                                                                                                                          
//!                                                                                                                                                                                              
//! ```                  
//!  
//!  
//!  In the diagram, you will see objects whose lifetime is static. The boxes for those have a double
//!  width margin. These are:
//!  
//!    - The `server` itself
//!    - The  global `TRACER`, which handles `log!` and `span!` and uses the global `PROCESSOR` to
//!     process the data constituting a trace `Span`s and log `Event`s
//!    - The global `EXPORTER`, which manages the `Storage` set of data structures, holding logs,
//!     traces (and capture settings) per request.
//!  
//!  Then, through the request lifecycle, different objects are created and dropped:
//!  
//!    - When a request comes in, its headers are processed and a [`Settings`] object is built, this
//!     object determines, for the request, how logging and tracing are going to be captured: if only
//!     traces, logs, or both, and which log levels are going to be captured.
//!    - Based on the settings, a new `Capturer` is created; a capturer is nothing but an exporter
//!     wrapped to start capturing / fetch the captures for this particular request.
//!  
//!  Then the capturing process works in this way:
//!    
//!    - The server receives a query **[1]**
//!    - It grabs the HTTP headers and builds a `Capture` object **[2]**, which is configured with the settings
//!      denoted by the `X-capture-telemetry`
//!    - Now the server tells the `Capturer` to start capturing all the logs and traces occurring on
//!     the request **[3]** (denoted by a `trace_id`) The `trace_id` is either carried on the `traceparent`
//!     header or implicitly created on the first span of the request. To _start capturing_ implies
//!     creating for the `trace_id` in two different data structures: `logs` and `traces`; and storing
//!     the settings for selecting the Spans and Event to capture **[4]**.
//!    - The server dispatches the request and _Somewhere_ else in the code, it is processed **[5]**.
//!    - There the code logs events and emits traces asynchronously, as part of processing **[6]**
//!    - Traces and Logs arrive at the `TRACER`, and get hydrated in the `PROCESSOR` **[7]**,
//!     and exported in batches by the `EXPORTER`**[8]** which writes them in the shard corresponding to
//!     the current `trace_id`, into the `logs` and `traces` storage **[9]**. The settings previously
//!     stored `trace_id` is used to pick which events and spans are going to be captured based on
//!     their level.
//!    - When the code that dispatches the request is done it returns a `PrismaResponse` to the
//!     server **[10]**.
//!    - Then the server asks the `Exporter` to fetch the captures **[11]**
//!    - The `Exporter` tells the `PROCESSOR` to flush any pending `Span`s and `Event`s **[12]**
//!    - And right after that, it fetches the captures from the `Storage` **[13]**. At that time, although
//!     that's not represented in the diagram, the captures are deleted from the storage, thus
//!     freeing any memory used for capturing during the request
//!    - Finally, the server sets the `logs` and `traces` extensions in the `PrismaResponse`**[14]**,
//!     it serializes the extended response in json format and returns it as an HTTP Response
//!     blob **[15]**.
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
pub(self) fn processor() -> SyncedSpanProcessor {
    PROCESSOR.to_owned()
}

/// Returns a clone to the global tracer used when capturing telemetry in the response
pub fn tracer() -> &'static sdk::trace::Tracer {
    &TRACER
}

/// Installs an opentelemetry tracer globally, which is configured to proecss
/// spans and export them to global exporter.
fn setup_and_install_tracer_globally() -> sdk::trace::Tracer {
    global::set_text_map_propagator(sdk::propagation::TraceContextPropagator::new());

    let provider_builder = sdk::trace::TracerProvider::builder().with_span_processor(processor());
    let provider = provider_builder.build();
    let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, "opentelemetry");

    global::set_tracer_provider(provider);
    tracer
}

mod capturer;
mod settings;
pub mod storage;
