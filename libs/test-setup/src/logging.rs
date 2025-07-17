use tracing_error::ErrorLayer;
use tracing_subscriber::{
    EnvFilter, FmtSubscriber,
    fmt::{
        TestWriter,
        format::{DefaultFields, Format},
    },
    layer::Layered,
    prelude::*,
};

pub(crate) fn init_logger() {
    tracing::subscriber::set_global_default(test_tracing_subscriber())
        .map_err(|err| {
            eprintln!("Error initializing the global logger: {err}");
            std::process::exit(1);
        })
        .ok();
}

type Sub = Layered<
    ErrorLayer<FmtSubscriber<DefaultFields, Format, EnvFilter, TestWriter>>,
    FmtSubscriber<DefaultFields, Format, EnvFilter, TestWriter>,
>;

fn test_tracing_subscriber() -> Sub {
    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .finish()
        .with(ErrorLayer::default())
}
