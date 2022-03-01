#![allow(clippy::wrong_self_convention, clippy::upper_case_acronyms, clippy::needless_borrow)]

#[macro_use]
extern crate tracing;

use query_engine::cli::CliCommand;
use query_engine::error::PrismaError;
use query_engine::logger::Logger;
use query_engine::opt::PrismaOpt;
use query_engine::server;
use query_engine::LogFormat;
use std::{error::Error, process};
use structopt::StructOpt;

type AnyError = Box<dyn Error + Send + Sync + 'static>;

#[tokio::main]
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
        logger.log_queries(opts.log_queries());
        logger.enable_telemetry(opts.open_telemetry);
        logger.telemetry_endpoint(&opts.open_telemetry_endpoint);

        logger.install().unwrap();

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
