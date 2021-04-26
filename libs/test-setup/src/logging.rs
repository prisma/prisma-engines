use tracing_error::ErrorLayer;
use tracing_subscriber::{
    fmt::format::{DefaultFields, Format},
    layer::Layered,
    prelude::*,
    EnvFilter, FmtSubscriber,
};

pub(crate) fn init_logger() {
    tracing::subscriber::set_global_default(test_tracing_subscriber())
        .map_err(|err| eprintln!("Error initializing the global logger: {}", err))
        .ok();
}

type Sub = Layered<
    ErrorLayer<FmtSubscriber<DefaultFields, Format, EnvFilter, PrintWriter>>,
    FmtSubscriber<DefaultFields, Format, EnvFilter, PrintWriter>,
>;

fn test_tracing_subscriber() -> Sub {
    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(PrintWriter)
        .finish()
        .with(ErrorLayer::default())
}

/// This is a temporary implementation detail for `tracing` logs in tests.
/// Instead of going through `std::io::stderr`, it goes through the specific
/// local stderr handle used by `eprintln` and `dbg`, allowing logs to appear in
/// specific test outputs for readability.
///
/// It is used from test_macros.
pub struct PrintWriter;

impl tracing_subscriber::fmt::MakeWriter for PrintWriter {
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
