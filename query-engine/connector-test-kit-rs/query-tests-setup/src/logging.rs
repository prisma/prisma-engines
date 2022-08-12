use query_engine_metrics::MetricRegistry;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    filter::Filtered, fmt::format::DefaultFields, layer::Layered, prelude::*, EnvFilter, Registry,
};

// Pretty ugly. I'm not sure how to make this better
type Sub = Layered<
    ErrorLayer<
        Layered<
            MetricRegistry,
            Layered<
                Filtered<
                    tracing_subscriber::fmt::Layer<
                        Registry,
                        DefaultFields,
                        tracing_subscriber::fmt::format::Format,
                        PrintWriter,
                    >,
                    EnvFilter,
                    Registry,
                >,
                Registry,
            >,
        >,
    >,
    Layered<
        MetricRegistry,
        Layered<
            Filtered<
                tracing_subscriber::fmt::Layer<
                    Registry,
                    DefaultFields,
                    tracing_subscriber::fmt::format::Format,
                    PrintWriter,
                >,
                EnvFilter,
                Registry,
            >,
            Registry,
        >,
    >,
    Layered<
        MetricRegistry,
        Layered<
            Filtered<
                tracing_subscriber::fmt::Layer<
                    Registry,
                    DefaultFields,
                    tracing_subscriber::fmt::format::Format,
                    PrintWriter,
                >,
                EnvFilter,
                Registry,
            >,
            Registry,
        >,
    >,
>;

pub fn test_tracing_subscriber(log_config: String, metrics: MetricRegistry) -> Sub {
    let filter = EnvFilter::new(log_config);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(PrintWriter)
        .with_filter(filter);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(metrics)
        .with(ErrorLayer::default())
}

/// This is a temporary implementation detail for `tracing` logs in tests.
/// Instead of going through `std::io::stderr`, it goes through the specific
/// local stderr handle used by `eprintln` and `dbg`, allowing logs to appear in
/// specific test outputs for readability.
///
/// It is used from test_macros.
pub struct PrintWriter;

impl tracing_subscriber::fmt::MakeWriter<'_> for PrintWriter {
    type Writer = PrintWriter;

    fn make_writer(&self) -> Self::Writer {
        PrintWriter
    }
}

impl std::io::Write for PrintWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        eprint!("{}", std::str::from_utf8(buf).unwrap_or("<invalid UTF-8>"));
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}
