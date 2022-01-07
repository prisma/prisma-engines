#![deny(rust_2018_idioms, unsafe_code)]

mod commands;
mod logger;

use migration_connector::ConnectorError;
use migration_core::rpc_api;
use std::sync::Arc;
use structopt::StructOpt;

/// When no subcommand is specified, the migration engine will default to starting as a JSON-RPC
/// server over stdio.
#[derive(Debug, StructOpt)]
#[structopt(version = env!("GIT_HASH"))]
struct MigrationEngineCli {
    /// Path to the datamodel
    #[structopt(short = "d", long, name = "FILE")]
    datamodel: Option<String>,
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

    let input = MigrationEngineCli::from_args();

    match input.cli_subcommand {
        None => start_engine(input.datamodel.as_deref()).await,
        Some(SubCommand::Cli(cli_command)) => {
            tracing::info!(git_hash = env!("GIT_HASH"), "Starting migration engine CLI");
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
    }));
}

struct JsonRpcHost {
    client: json_rpc_stdio::Client,
}

#[async_trait::async_trait]
impl migration_connector::ConnectorHost for JsonRpcHost {
    async fn print(&self, text: &str) -> migration_connector::ConnectorResult<()> {
        let notification = serde_json::json!({ "content": text });

        self.client
            .notify("print".to_owned(), notification)
            .await
            .map_err(|err| ConnectorError::from_source(err, "JSON-RPC error"))
    }
}

async fn start_engine(datamodel_location: Option<&str>) {
    use std::io::Read as _;

    tracing::info!(git_hash = env!("GIT_HASH"), "Starting migration engine RPC server",);

    let datamodel = datamodel_location.map(|location| {
        let mut file = std::fs::File::open(location).expect("error opening datamodel file");
        let mut datamodel = String::new();
        file.read_to_string(&mut datamodel).unwrap();
        datamodel
    });

    let (client, adapter) = json_rpc_stdio::new_client();
    let host = JsonRpcHost { client };

    let api = rpc_api(datamodel, Arc::new(host));
    // Block the thread and handle IO in async until EOF.
    json_rpc_stdio::run_with_client(&api, adapter).await.unwrap();
}
