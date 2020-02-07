pub(crate) fn init_logger() {
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init()
}
