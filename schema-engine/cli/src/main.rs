#![deny(rust_2018_idioms, unsafe_code)]

mod commands;
mod json_rpc_stdio;
mod logger;

use std::{
    backtrace::Backtrace,
    sync::{Arc, LazyLock},
    time::Duration,
};

use schema_connector::{BoxFuture, ConnectorHost, ConnectorResult};
use schema_core::RpcApi;
use structopt::StructOpt;
use tokio::{signal, sync::oneshot};
use tokio_util::sync::CancellationToken;

/// The timeout for graceful shutdown of asynchronous tasks like network connections on SIGTERM.
static GRACEFUL_SHUTDOWN_TIMEOUT: LazyLock<Duration> = LazyLock::new(|| {
    std::env::var("PRISMA_GRACEFUL_SHUTDOWN_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_secs(4))
});

/// The shutdown deadline for blocking background tasks.
static BLOCKING_TASKS_SHUTDOWN_TIMEOUT: LazyLock<Duration> = LazyLock::new(|| {
    std::env::var("PRISMA_BLOCKING_TASKS_SHUTDOWN_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(200))
});

/// When no subcommand is specified, the schema engine will default to starting as a JSON-RPC
/// server over stdio.
#[derive(Debug, StructOpt)]
#[structopt(version = env!("GIT_HASH"))]
struct SchemaEngineCli {
    /// List of paths to the Prisma schema files.
    #[structopt(short = "d", long, name = "FILE")]
    datamodels: Option<Vec<String>>,
    #[structopt(subcommand)]
    cli_subcommand: Option<SubCommand>,
}

#[derive(Debug, StructOpt)]
enum SubCommand {
    /// Doesn't start a server, but allows running specific commands against Prisma.
    #[structopt(name = "cli")]
    Cli(commands::Cli),
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Tokio runtime");

    rt.block_on(async_main());

    // By default (i.e. on Drop), Tokio runs the pending async tasks until they
    // yield and blocking tasks until completion. The JSON-RPC server uses
    // blocking file I/O to read from stdin, and spawns a background blocking
    // task for it, which keeps running even if the future is cancelled. This
    // means that the process would keep running forever until the stdin is
    // closed externally (i.e. from JavaScript) or the process is terminated
    // with a signal which we don't/can't handle (e.g. SIGKILL). That's why we
    // need to shutdown the runtime explicitly, providing a timeout for pending
    // blocking tasks.
    rt.shutdown_timeout(*BLOCKING_TASKS_SHUTDOWN_TIMEOUT);
}

async fn async_main() {
    set_panic_hook();
    logger::init_logger();

    let input = SchemaEngineCli::from_args();
    let shutdown_token = CancellationToken::new();
    let (done_tx, done_rx) = oneshot::channel();

    let work = tokio::spawn({
        let shutdown_token = shutdown_token.clone();
        async {
            match input.cli_subcommand {
                None => start_engine(input.datamodels, shutdown_token).await,
                Some(SubCommand::Cli(cli_command)) => {
                    tracing::info!(git_hash = env!("GIT_HASH"), "Starting schema engine CLI");
                    cli_command.run(shutdown_token).await;
                }
            }
            _ = done_tx.send(());
        }
    });

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

    let graceful_shutdown = async {
        shutdown_token.cancel();

        match tokio::time::timeout(*GRACEFUL_SHUTDOWN_TIMEOUT, work).await {
            Ok(Ok(())) => (),
            Ok(Err(err)) => {
                panic!("main task panicked: {err}");
            }
            Err(_) => {
                tracing::error!("Graceful shutdown timed out");
                std::process::exit(1);
            }
        }
    };

    tokio::select! {
        _ = done_rx => (),

        _ = interrupt => {
            tracing::info!("Received SIGINT/Ctrl+C");
            graceful_shutdown.await;
        }

        _ = terminate => {
            tracing::info!("Received SIGTERM");
            graceful_shutdown.await;
        }
    }
}

fn set_panic_hook() {
    std::panic::set_hook(Box::new(move |panic_info| {
        let message = panic_utils::downcast_ref_to_string(panic_info.payload()).unwrap_or("<unknown panic>");

        let location = panic_info
            .location()
            .map(|loc| loc.to_string())
            .unwrap_or_else(|| "<unknown location>".to_owned());

        tracing::error!(
            is_panic = true,
            backtrace = ?Backtrace::force_capture(),
            location = %location,
            "[{}] {}",
            location,
            message
        );
        std::process::exit(101);
    }));
}

struct JsonRpcHost {
    client: json_rpc_stdio::Client,
}

impl ConnectorHost for JsonRpcHost {
    fn print<'a>(&'a self, text: &'a str) -> BoxFuture<'a, ConnectorResult<()>> {
        Box::pin(async move {
            // Adapter to be removed when https://github.com/prisma/prisma/issues/11761 is closed.
            assert!(!text.is_empty());
            assert!(text.ends_with('\n'));
            let text = &text[..text.len() - 1];

            let notification = serde_json::json!({ "content": text });

            let _: std::collections::HashMap<(), ()> =
                self.client.call("print".to_owned(), notification).await.unwrap();
            Ok(())
        })
    }
}

async fn start_engine(datamodel_locations: Option<Vec<String>>, shutdown_token: CancellationToken) {
    use std::io::Read as _;

    tracing::info!(git_hash = env!("GIT_HASH"), "Starting schema engine RPC server",);

    let datamodel_locations = datamodel_locations.map(|datamodel_locations| {
        datamodel_locations
            .into_iter()
            .map(|location| {
                let mut file = match std::fs::File::open(&location) {
                    Ok(file) => file,
                    Err(e) => panic!("Error opening datamodel file in `{location}`: {e}"),
                };

                let mut datamodel = String::new();

                if let Err(e) = file.read_to_string(&mut datamodel) {
                    panic!("Error reading datamodel file `{location}`: {e}");
                };

                (location, datamodel)
            })
            .collect::<Vec<_>>()
    });

    let (client, adapter) = json_rpc_stdio::new_client();
    let host = JsonRpcHost { client };

    let api = RpcApi::new(datamodel_locations, Arc::new(host));

    // Handle IO in async until EOF or cancelled. Note that the even if the
    // [`json_rpc_stdio::run_with_client`] future is cancelled, a separate
    // worker thread may still be blocked on read in a blocking task unless we
    // read EOF. This is why we need to explicitly use
    // [`tokio::runtime::Runtime::shutdown_timeout`] to shut down the runtime
    // instead of relying on the default behavior.
    tokio::select! {
        result = json_rpc_stdio::run_with_client(api.io_handler(), adapter) => result.unwrap(),
        _ = shutdown_token.cancelled() => (),
    }

    api.dispose().await.unwrap();
}
