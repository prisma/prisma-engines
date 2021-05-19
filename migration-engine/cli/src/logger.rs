use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;

pub(crate) fn init_logger() {
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .with_timer(tracing_subscriber::fmt::time::ChronoUtc::rfc3339())
        .with_writer(std::io::stderr)
        .finish()
        .with(ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|err| eprintln!("Error initializing the global logger: {}", err))
        .ok();
}
