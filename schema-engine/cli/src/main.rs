#![deny(rust_2018_idioms, unsafe_code)]

mod commands;
mod json_rpc_stdio;
mod logger;

use schema_connector::{BoxFuture, ConnectorHost, ConnectorResult};
use schema_core::rpc_api;
use std::sync::Arc;
use structopt::StructOpt;

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

#[tokio::main]
async fn main() {
    set_panic_hook();
    logger::init_logger();

    let input = SchemaEngineCli::from_args();

    match input.cli_subcommand {
        None => start_engine(input.datamodels).await,
        Some(SubCommand::Cli(cli_command)) => {
            tracing::info!(git_hash = env!("GIT_HASH"), "Starting schema engine CLI");
            cli_command.run().await;
        }
    }
}

fn set_panic_hook() {
    std::panic::set_hook(Box::new(move |panic_info| {
        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| panic_info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<unknown panic>");

        let location = panic_info
            .location()
            .map(|loc| loc.to_string())
            .unwrap_or_else(|| "<unknown location>".to_owned());

        tracing::error!(
            is_panic = true,
            backtrace = ?backtrace::Backtrace::new(),
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

async fn start_engine(datamodel_locations: Option<Vec<String>>) {
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

    let api = rpc_api(datamodel_locations, Arc::new(host));
    // Block the thread and handle IO in async until EOF.
    json_rpc_stdio::run_with_client(&api, adapter).await.unwrap();
}
