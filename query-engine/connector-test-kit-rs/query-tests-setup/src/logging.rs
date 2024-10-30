use query_core::telemetry::helpers as telemetry_helpers;
use tracing::Subscriber;
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::*, Layer};

use crate::LogEmit;

pub fn test_tracing_subscriber(log_config: String, log_tx: LogEmit) -> impl Subscriber {
    let filter = telemetry_helpers::env_filter(true, telemetry_helpers::QueryEngineLogLevel::Override(log_config));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(PrintWriter::new(log_tx))
        .with_filter(filter);

    tracing_subscriber::registry()
        .with(fmt_layer.boxed())
        .with(ErrorLayer::default())
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
        eprint!("{log}");
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}
