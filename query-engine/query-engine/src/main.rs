#[macro_use]
extern crate tracing;

use cli::CliCommand;
use error::PrismaError;
use opt::PrismaOpt;
use request_handlers::PrismaResponse;
use std::{error::Error, process};
use structopt::StructOpt;
use tracing::subscriber;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use tide_server_timing::TimingLayer;
use tracing_subscriber::layer::SubscriberExt;

mod cli;
mod context;
mod dmmf;
mod error;
mod exec_loader;
mod opt;
mod request_handlers;
mod server;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum LogFormat {
    Text,
    Json,
}

pub type PrismaResult<T> = Result<T, PrismaError>;
type AnyError = Box<dyn Error + Send + Sync + 'static>;

#[async_std::main]
async fn main() -> Result<(), AnyError> {
    return main().await.map_err(|err| {
        info!("Encountered error during initialization:");
        err.render_as_json().expect("error rendering");
        process::exit(1)
    });

    async fn main() -> Result<(), PrismaError> {
        let opts = PrismaOpt::from_args();
        init_logger(opts.log_format());
        feature_flags::initialize(opts.raw_feature_flags.as_slice())?;
        match CliCommand::from_opt(&opts)? {
            Some(cmd) => cmd.execute().await?,
            None => {
                set_panic_hook(opts.log_format());
                server::listen(opts).await?;
            }
        }
        Ok(())
    }
}

fn init_logger(log_format: LogFormat) {
    // Enable `tide` logs to be captured.
    let filter = EnvFilter::from_default_env().add_directive("tide=info".parse().unwrap());

    match log_format {
        LogFormat::Text => {
            let subscriber = FmtSubscriber::builder()
                .with_max_level(tracing::Level::TRACE)
                .finish()
                .with(TimingLayer::new());

            subscriber::set_global_default(subscriber).expect("Could not initialize logger");
        }
        LogFormat::Json => {
            let subscriber = FmtSubscriber::builder()
                .json()
                .with_env_filter(filter)
                .finish()
                .with(TimingLayer::new());
            subscriber::set_global_default(subscriber).expect("Could not initialize logger");
        }
    }
}

fn set_panic_hook(log_format: LogFormat) {
    if let LogFormat::Json = log_format {
        std::panic::set_hook(Box::new(|info| {
            let payload = info
                .payload()
                .downcast_ref::<String>()
                .map(Clone::clone)
                .unwrap_or_else(|| info.payload().downcast_ref::<&str>().unwrap().to_string());

            match info.location() {
                Some(location) => {
                    tracing::event!(
                        tracing::Level::ERROR,
                        message = "PANIC",
                        reason = payload.as_str(),
                        file = location.file(),
                        line = location.line(),
                        column = location.column(),
                    );
                }
                None => {
                    tracing::event!(tracing::Level::ERROR, message = "PANIC", reason = payload.as_str());
                }
            }

            std::process::exit(255);
        }));
    }
}
