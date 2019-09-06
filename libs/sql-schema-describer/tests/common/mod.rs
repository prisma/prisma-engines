use log::LevelFilter;
use std::sync::atomic::{AtomicBool, Ordering};

pub const SCHEMA: &str = "DatabaseInspector-Test";

static IS_SETUP: AtomicBool = AtomicBool::new(false);

pub fn setup() {
    let is_setup = IS_SETUP.load(Ordering::Relaxed);
    if is_setup {
        return;
    }

    let log_level = match std::env::var("TEST_LOG")
        .unwrap_or("warn".to_string())
        .to_lowercase()
        .as_ref()
    {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Warn,
    };
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!("[{}][{}] {}", record.target(), record.level(), message))
        })
        .level(log_level)
        .chain(std::io::stdout())
        .apply()
        .expect("fern configuration");

    IS_SETUP.store(true, Ordering::Relaxed);
}
