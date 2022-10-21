use query_engine_metrics::MetricRegistry;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::Layered, prelude::*, EnvFilter, Layer, Registry};

use crate::LogEmit;

// Pretty ugly. I'm not sure how to make this better
type Sub = Layered<
    ErrorLayer<
        Layered<
            Box<
                dyn tracing_subscriber::Layer<
                        Layered<Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>, Registry>,
                    > + Send
                    + Sync,
            >,
            Layered<Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>, Registry>,
        >,
    >,
    Layered<
        Box<
            dyn tracing_subscriber::Layer<Layered<Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>, Registry>>
                + Send
                + Sync,
        >,
        Layered<Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>, Registry>,
    >,
>;

pub fn test_tracing_subscriber(log_config: &str, metrics: MetricRegistry, log_tx: LogEmit) -> Sub {
    let filter = create_env_filter(true, log_config);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(PrintWriter::new(log_tx))
        .with_filter(filter);

    tracing_subscriber::registry()
        .with(fmt_layer.boxed())
        .with(metrics.boxed())
        .with(ErrorLayer::default())
}

fn create_env_filter(log_queries: bool, qe_log_level: &str) -> EnvFilter {
    let mut filter = EnvFilter::from_default_env()
        .add_directive("tide=error".parse().unwrap())
        .add_directive("tonic=error".parse().unwrap())
        .add_directive("h2=error".parse().unwrap())
        .add_directive("hyper=error".parse().unwrap())
        .add_directive("tower=error".parse().unwrap());

    filter = filter
        .add_directive(format!("query_engine={}", &qe_log_level).parse().unwrap())
        .add_directive(format!("query_core={}", &qe_log_level).parse().unwrap())
        .add_directive(format!("query_connector={}", &qe_log_level).parse().unwrap())
        .add_directive(format!("sql_query_connector={}", &qe_log_level).parse().unwrap())
        .add_directive(format!("mongodb_query_connector={}", &qe_log_level).parse().unwrap());

    if log_queries {
        filter = filter.add_directive("quaint[{is_query}]=trace".parse().unwrap());
    }

    filter
}

/// This is a temporary implementation detail for `tracing` logs in tests.
/// Instead of going through `std::io::stderr`, it goes through the specific
/// local stderr handle used by `eprintln` and `dbg`, allowing logs to appear in
/// specific test outputs for readability.
///
/// It is used from test_macros.
pub struct PrintWriter {
    tx: LogEmit,
}

impl PrintWriter {
    fn new(tx: LogEmit) -> Self {
        Self { tx }
    }
}

impl tracing_subscriber::fmt::MakeWriter<'_> for PrintWriter {
    type Writer = PrintWriter;

    fn make_writer(&self) -> Self::Writer {
        PrintWriter::new(self.tx.clone())
    }
}

impl std::io::Write for PrintWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        let log = std::str::from_utf8(buf).unwrap_or("<invalid UTF-8>");

        if log.contains("quaint") || log.contains("mongodb_query_connector") {
            let plain_bytes = strip_ansi_escapes::strip(buf)?;
            let plain_log = std::str::from_utf8(&plain_bytes).unwrap_or("<invalid UTF-8>");
            let _ = self.tx.send(plain_log.to_string());
        }
        eprint!("{}", log);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}
