#![deny(rust_2018_idioms)]
#![deny(unsafe_code)]

mod commands;
#[cfg(test)]
mod error_tests;
mod logger;

use migration_core::{api::RpcApi, error::Error as CoreError};
use structopt::StructOpt;

/// When no subcommand is specified, the migration engine will default to starting as a JSON-RPC
/// server over stdio.
#[derive(Debug, StructOpt)]
#[structopt(version = env!("GIT_HASH"))]
struct MigrationEngineCli {
    /// Run only a single command, then exit
    #[structopt(short = "s", long)]
    single_cmd: bool,
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

impl SubCommand {
    #[cfg(test)]
    fn unwrap_cli(self) -> commands::Cli {
        match self {
            SubCommand::Cli(cli) => cli,
        }
    }
}

#[tokio::main]
async fn main() {
    user_facing_errors::set_panic_hook();
    logger::init_logger();

    let input = MigrationEngineCli::from_args();

    match input.cli_subcommand {
        None => {
            if let Some(datamodel_location) = input.datamodel.as_ref() {
                start_engine(datamodel_location, input.single_cmd).await
            } else {
                panic!("Missing --datamodel");
            }
        }
        Some(SubCommand::Cli(cli_command)) => {
            tracing::info!(git_hash = env!("GIT_HASH"), "Starting migration engine CLI");
            cli_command.run().await;
        }
    }
}

async fn start_engine(datamodel_location: &str, single_cmd: bool) -> ! {
    use std::io::Read as _;

    tracing::info!(git_hash = env!("GIT_HASH"), "Starting migration engine RPC server",);
    let mut file = std::fs::File::open(datamodel_location).expect("error opening datamodel file");

    let mut datamodel = String::new();
    file.read_to_string(&mut datamodel).unwrap();

    if single_cmd {
        let api = RpcApi::new(&datamodel).await.unwrap();
        let response = api.handle().unwrap();

        println!("{}", response);
    } else {
        match RpcApi::new(&datamodel).await {
            // Block the thread and handle IO in async until EOF.
            Ok(api) => json_rpc_stdio::run(api.io_handler()).await.unwrap(),
            Err(err) => {
                let (error, exit_code) = match &err {
                    CoreError::DatamodelError(errors) => {
                        let error = user_facing_errors::UnknownError {
                            message: migration_core::api::pretty_print_datamodel_errors(errors, &datamodel)
                                .expect("rendering error"),
                            backtrace: Some(format!("{:?}", user_facing_errors::new_backtrace())),
                        };

                        (user_facing_errors::Error::from(error), 1)
                    }
                    _ => (migration_core::api::render_error(err), 255),
                };

                serde_json::to_writer(std::io::stdout().lock(), &error).expect("failed to write to stdout");
                std::process::exit(exit_code)
            }
        }
    }

    std::process::exit(0);
}
