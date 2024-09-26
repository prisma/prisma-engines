#![allow(clippy::upper_case_acronyms)]

use query_engine::cli::CliCommand;
use query_engine::context;
use query_engine::error::PrismaError;
use query_engine::opt::PrismaOpt;
use query_engine::server;
use query_engine::LogFormat;
use std::{error::Error, process};
use structopt::StructOpt;

type AnyError = Box<dyn Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    return main().await.map_err(|err| {
        tracing::info!("Encountered error during initialization:");
        err.render_as_json().expect("error rendering");
        process::exit(1)
    });

    async fn main() -> Result<(), PrismaError> {
        let opts = PrismaOpt::from_args();

        match CliCommand::from_opt(&opts)? {
            Some(cmd) => cmd.execute().await?,
            None => {
                let cx = context::setup(&opts, true, None).await?;
                set_panic_hook(opts.log_format());
                server::listen(cx, &opts).await?;
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
                .cloned()
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
