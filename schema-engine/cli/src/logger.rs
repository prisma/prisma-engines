use schema_connector::ConnectorError;
use schema_core::TimingsLayer;
use tracing_error::ErrorLayer;

pub(crate) fn init_logger() {
    use tracing_subscriber::{EnvFilter, FmtSubscriber, prelude::*};

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .with_writer(std::io::stderr)
        .finish()
        .with(ErrorLayer::default())
        .with(TimingsLayer);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|err| eprintln!("Error initializing the global logger: {err}"))
        .ok();
}

pub(crate) fn log_error_and_exit(error: ConnectorError) -> ! {
    let message: &dyn std::fmt::Display = match error.known_error() {
        Some(known_error) => &known_error.message,
        _ => &error,
    };

    tracing::error!(
        is_panic = false,
        error_code = error.error_code().unwrap_or(""),
        message = %message,
    );

    std::process::exit(1)
}
