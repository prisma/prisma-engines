#![allow(clippy::upper_case_acronyms)]

use query_engine::cli::CliCommand;
use query_engine::context;
use query_engine::error::PrismaError;
use query_engine::opt::PrismaOpt;
use query_engine::server;
use query_engine::LogFormat;
use structopt::StructOpt;
use tokio::{select, signal};

#[tokio::main]
async fn main() {
    let work = async {
        let opts = PrismaOpt::from_args();

        match CliCommand::from_opt(&opts)? {
            Some(cmd) => cmd.execute().await?,
            None => {
                let cx = context::setup(&opts).await?;
                set_panic_hook(opts.log_format());
                server::listen(cx, &opts).await?;
            }
        }

        Result::<(), PrismaError>::Ok(())
    };

    let interrupt = async { signal::ctrl_c().await.expect("failed to listen for SIGINT/Ctrl+C") };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending();

    select! {
        result = work => {
            if let Err(err) = result {
                tracing::info!("Encountered error during initialization:");
                err.render_as_json().expect("failed to render error");
                std::process::exit(1);
            }
        }
        _ = interrupt => {
            tracing::info!("Received SIGINT/Ctrl+C, shutting down");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, shutting down");
        }
    }
}

fn set_panic_hook(log_format: LogFormat) {
    if let LogFormat::Json = log_format {
        std::panic::set_hook(Box::new(|info| {
            let payload = panic_utils::downcast_ref_to_string(info.payload()).unwrap_or_default();

            match info.location() {
                Some(location) => {
                    tracing::event!(
                        tracing::Level::ERROR,
                        message = "PANIC",
                        reason = payload,
                        file = location.file(),
                        line = location.line(),
                        column = location.column(),
                    );
                }
                None => {
                    tracing::event!(tracing::Level::ERROR, message = "PANIC", reason = payload);
                }
            }
        }));
    }
}
