#![allow(clippy::wrong_self_convention)]

#[macro_use]
extern crate tracing;

use cli::CliCommand;
use error::PrismaError;
use logger::Logger;
use opt::PrismaOpt;
use std::{error::Error, process};
use structopt::StructOpt;

mod cli;
mod context;
mod error;
mod logger;
mod opt;
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

        let mut logger = Logger::new("query-engine-http");
        logger.log_format(opts.log_format());
        logger.enable_telemetry(opts.open_telemetry);
        logger.telemetry_endpoint(&opts.open_telemetry_endpoint);

        // The guard needs to be in scope for the whole lifetime of the service.
        let _guard = logger.install().unwrap();

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
