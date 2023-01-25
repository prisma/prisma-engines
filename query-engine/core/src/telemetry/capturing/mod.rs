//! Telemetry Capturing is the process of recording the logs and traces happening during a request
//! to the binary engine, and rendering them in the response.
//!
//! The interaction diagram below (soorry width!) shows the different roles at play during telemetry
//! capturing. A textual explanatation follows it. For the sake of example a server environment
//! --the query-engine crate-- is assumed.
//! #                                                           ╔═══════════════════════╗     ╔═══════════════╗                                                                     
//! #                                                           ║<<SpanProcessor, Sync>>║     ║    Storage    ║   ╔═══════════════════╗                                             
//! #      ┌───────────────────┐                                ║       PROCESSOR       ║     ║               ║   ║      TRACER       ║                                             
//! #      │      Server       │                                ╚═══════════╦═══════════╝     ╚═══════╦═══════╝   ╚═════════╦═════════╝                                             
//! #      └─────────┬─────────┘                                            │                         │                     │                                                       
//! #                │                                                      │                         │                     │                                                       
//! #                │                                                      │                         │                     │                                                       
//! #      POST      │                                                      │                         │                     │                                                       
//! # (body, headers)│                                                      │                         │                     │                                                       
//! #    ──────────▶┌┴┐                                                     │                         │                     │                                                       
//! #        ┌─┐    │ │new(headers)╔════════════╗                           │                         │                     │                                                       
//! #        │1│    │ ├───────────▶║s: Settings ║                           │                         │                     │                                                       
//! #        └─┘    │ │            ╚════════════╝                           │                         │                     │                                                       
//! #               │ │                                                     │                         │                     │                                                       
//! #               │ │                    ╔═══════════════════╗            │                         │                     │                                                       
//! #               │ │                    ║ Capturer::Enabled ║            │                         │                     │                     ┌────────────┐                    
//! #               │ │                    ╚═══════════════════╝            │                         │                     │                     │<<Somewhere>│                    
//! #               │ │                              │                      │                         │                     │                     └──────┬─────┘                    
//! #               │ │   ┌─┐  new(trace_id, s)      │                      │                         │                     │                            │                          
//! #               │ ├───┤2├───────────────────────▶│                      │                         │                     │                            │                          
//! #               │ │   └─┘                        │                      │                         │                     │                            │                          
//! #               │ │                              │                      │                         │                     │                            │                          
//! #               │ │   ┌─┐ start_capturing()      │   start_capturing    │                         │                     │                            │                          
//! #               │ ├───┤3├───────────────────────▶│    (trace_id, s)     │                         │                     │                            │                          
//! #               │ │   └─┘                        │                      │                         │                     │                            │                          
//! #               │ │                              ├─────────────────────▶│                         │                     │                            │                          
//! #               │ │                              │                      │                         │                     │                            │                          
//! #               │ │                              │                      │ ┌─┐insert(trace_id, s)  │                     │                            │                          
//! #               │ │                              │                      ├─┤4├────────────────────▶│                     │                            │                          
//! #               │ │                              │                      │ └─┘                     │                     │                            │                          
//! #               │ │                              │                      │                         │            ┌─┐      │          process_query     │                          
//! #               │ ├──────────────────────────────┼──────────────────────┼─────────────────────────┼────────────┤5├──────┼──────────────────────────▶┌┴┐                         
//! #               │ │                              │                      │                         │            └─┘      │                           │ │                         
//! #               │ │                              │                      │                         │                     │                           │ │                         
//! #               │ │                              │                      │                         │                     │                           │ │  ┌─────────────────────┐
//! #               │ │                              │                      │                         │                     │     log! / span!     ┌─┐  │ │  │ res: PrismaResponse │
//! #               │ │                              │                      │                         │                     │◀─────────────────────┤6├──│ │  └──────────┬──────────┘
//! #               │ │                              │                      │        on_end(span_data)│            ┌─┐      │                      └─┘  │ │   new       │           
//! #               │ │                              │                      │◀────────────────────────┼────────────┤7├──────┤                           │ │────────────▶│           
//! #               │ │                              │                      │                         │            └─┘      │                           │ │             │           
//! #               │ │                              │                      │  append(trace_id, logs, │                     │                           │ │             │           
//! #               │ │                              │                      │  ┌─┐     traces)        │                     │                           │ │             │           
//! #               │ │                              │                      ├──┤8├───────────────────▶│                     │                           │ │             │           
//! #               │ │                              │                      │  └─┘                    │                     │                           │ │             │           
//! #               │ │      res: PrismaResponse     │     ┌─┐              │                         │                     │                           │ │             │           
//! #               │ │◁ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┼ ─ ─ ┤9├ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─└┬┘             │           
//! #               │ │ ┌────┐ fetch_captures()      │     └─┘              │                         │                     │                            │              │           
//! #               │ ├─┤ 10 ├──────────────────────▶│      fetch_captures  │                         │                     │                            │              │           
//! #               │ │ └────┘                       │        (trace_id)    │                         │                     │                            │              │           
//! #               │ │                              ├─────────────────────▶│                         │                     │                            x              │           
//! #               │ │                              │                      │                         │                     │                                           │           
//! #               │ │                              │                      │┌────┐  get logs/traces  │                     │                                           │           
//! #               │ │                              │                      ├┤ 11 ├───────────────────▶                     │                                           │           
//! #               │ │                              │                      │└────┘                   │                     │                                           │           
//! #               │ │                              │                      │◁ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│                     │                                           │           
//! #               │ │                              │                      │                         │                     │                                           │           
//! #               │ │                              ◁ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│                         │                     │                                           │           
//! #               │ │          logs, traces        │                      │                         │                     │                                           │           
//! #               │ │◁─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─│                      │                         │                     │                                           │           
//! #               │ │                              x        ┌────┐        │                         │                     │                res.set_extension(logs)    │           
//! #               │ ├───────────────────────────────────────┤ 12 ├────────┼─────────────────────────┼─────────────────────┼──────────────────────────────────────────▶│           
//! #               │ │                                       └────┘        │                         │                     │                res.set_extension(traces)  │           
//! #               │ ├─────────────────────────────────────────────────────┼─────────────────────────┼─────────────────────┼──────────────────────────────────────────▶│           
//! #        ◀ ─ ─ ─└┬┘                                                     │                         │                     │                                           x           
//! #     json!(res) │                                                                                                                                                              
//! #        ┌────┐  │                                                                                                                                                              
//! #        │ 13 │  │                                                                                                                                                              
//! #        └────┘                                                                                                                                                                 
//! #                                                                                                                                                                               
//! #                                                                          ◁─ ─ ─ ─ return                                                                                      
//! #                                                                                                                                                                               
//! #                                                                          ◁─────── call (pseudo-signatures)                                                                    
//! #                                                                                                                                                                                                                                                                                                                                                         
//!  
//!  In the diagram, you will see objects whose lifetime is static. The boxes for those have a double
//!  width margin. These are:
//!  
//!    - The `server` itself
//!    - The  global `TRACER`, which handles `log!` and `span!` and uses the global `PROCESSOR` to
//!     process the data constituting a trace `Span`s and log `Event`s
//!    - The global `PROCESSOR`, which manages the `Storage` set of data structures, holding logs,
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
//!    - There the code logs events and emits traces asynchronously, as part of the processing **[6]**
//!    - Traces and Logs arrive at the `TRACER`, and get hydrated in the `PROCESSOR` **[7]**,
//!     which writes them in the shard corresponding to the current `trace_id`, into the
//!     `logs` and `traces` storage **[8]**. The settings previously stored under the `trace_id`
//!     key, are used to pick which events and spans are going to be captured based on their level.
//!    - When the code that dispatches the request is done it returns a `PrismaResponse` to the
//!     server **[9]**.
//!    - Then the server asks the `PROCESSOR` to fetch the captures **[10]**
//!    - And right after that, it fetches the captures from the `Storage` **[11]**. At that time, although
//!     that's not represented in the diagram, the captures are deleted from the storage, thus
//!     freeing any memory used for capturing during the request
//!    - Finally, the server sets the `logs` and `traces` extensions in the `PrismaResponse`**[12]**,
//!     it serializes the extended response in json format and returns it as an HTTP Response
//!     blob **[13]**.
//!  
pub use self::capturer::Capturer;
pub use self::settings::Settings;

use self::capturer::Processor;
use once_cell::sync::Lazy;
use opentelemetry::{global, sdk, trace};
use query_engine_metrics::MetricRegistry;
use tracing::subscriber;
use tracing_subscriber::{
    filter::filter_fn, layer::Layered, prelude::__tracing_subscriber_SubscriberExt, Layer, Registry,
};

static PROCESSOR: Lazy<capturer::Processor> = Lazy::new(Processor::default);

/// Creates a new capturer, which is configured to export traces and log events happening during a
/// particular request
pub fn capturer(trace_id: trace::TraceId, settings: Settings) -> Capturer {
    Capturer::new(PROCESSOR.to_owned(), trace_id, settings)
}

/// Adds a capturing layer to the given subscriber and installs the transformed subscriber as the
/// global, default subscriber
pub fn install_capturing_layer(
    subscriber: Layered<Option<MetricRegistry>, Layered<Box<dyn Layer<Registry> + Send + Sync>, Registry>>,
    log_queries: bool,
) {
    // set a trace context propagator, so that the trace context is propagated via the
    // `traceparent` header from other systems
    global::set_text_map_propagator(sdk::propagation::TraceContextPropagator::new());
    // create a tracer provider that is configured to use our custom processor to process spans
    let provider = sdk::trace::TracerProvider::builder()
        .with_span_processor(PROCESSOR.to_owned())
        .build();
    // create a tracer out of the provider
    let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, "opentelemetry");
    // set the provider as the global provider
    global::set_tracer_provider(provider);
    // create a layer that will filter initial events and spans based on the log level configuration
    // from the environment and a specific filter to discard things that we are not interested in
    // from a capturiong perspective
    let telemetry_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_filter(crate::helpers::env_filter(
            log_queries,
            crate::helpers::QueryEngineLogLevel::FromEnv,
        ))
        .with_filter(filter_fn(helpers::span_and_event_filter));
    // decorate the given subscriber (more layers were added before this one) with the telemetry layer
    let subscriber = subscriber.with(telemetry_layer);
    // and finally set the subscriber as the global, default subscriber
    subscriber::set_global_default(subscriber).unwrap();
}

mod capturer;
mod helpers;
mod settings;
pub mod storage;
