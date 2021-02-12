use tracing::subscriber;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

pub fn init() {
    let filter = EnvFilter::from_default_env();
    let subscriber = FmtSubscriber::builder().json().with_env_filter(filter).finish();
    let _ = subscriber::set_global_default(subscriber);
}
