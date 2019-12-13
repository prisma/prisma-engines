use tracing_subscriber::*;

type Subscriber<T> =
    FmtSubscriber<tracing_subscriber::fmt::format::NewRecorder, tracing_subscriber::fmt::format::Format, EnvFilter, T>;

pub fn test_tracing_subscriber<T>(log_config: &'static str) -> Subscriber<impl Fn() -> PrintWriter> {
    let filter = EnvFilter::new(log_config);

    FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_writer(print_writer)
        .finish()
}

/// This is a temporary implementation detail for `tracing` logs in tests.
/// Instead of going through `std::io::stderr`, it goes through the specific
/// local stderr handle used by `eprintln` and `dbg`, allowing logs to appear in
/// specific test outputs for readability.
///
/// It is used from test_macros.
pub fn print_writer() -> PrintWriter {
    PrintWriter
}

/// See `print_writer`.
pub struct PrintWriter;

impl std::io::Write for PrintWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        eprint!("{}", std::str::from_utf8(buf).unwrap_or("<invalid UTF-8>"));
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}
