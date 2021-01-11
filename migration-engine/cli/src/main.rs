#![deny(rust_2018_idioms, unsafe_code)]

mod commands;
#[cfg(test)]
mod error_tests;
mod logger;

use enumflags2::BitFlags;
use migration_connector::MigrationFeature;
use migration_core::{api::RpcApi, CoreError};
use std::io;
use structopt::StructOpt;

/// When no subcommand is specified, the migration engine will default to starting as a JSON-RPC
/// server over stdio.
#[derive(Debug, StructOpt)]
#[structopt(version = env!("GIT_HASH"))]
struct MigrationEngineCli {
    /// Path to the datamodel
    #[structopt(short = "d", long, name = "FILE")]
    datamodel: Option<String>,
    /// A list of blocked preview features to enable (`all` enables everything).
    #[structopt(long, use_delimiter = true)]
    enabled_preview_features: Vec<String>,
    #[structopt(subcommand)]
    cli_subcommand: Option<SubCommand>,
}

impl MigrationEngineCli {
    pub fn preview_feature_flags(&self) -> BitFlags<MigrationFeature> {
        let mut enabled_features = BitFlags::empty();

        for feature in self.enabled_preview_features.iter() {
            if feature == "all" {
                return BitFlags::all();
            }

            let feature: io::Result<MigrationFeature> = feature.parse();

            match feature {
                Ok(feature) => enabled_features.insert(feature),
                Err(e) => tracing::debug!("{}", e),
            }
        }

        enabled_features
    }
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
    let features = input.preview_feature_flags();

    match input.cli_subcommand {
        None => {
            if let Some(datamodel_location) = input.datamodel.as_ref() {
                start_engine(datamodel_location, input.preview_feature_flags()).await
            } else {
                panic!("Missing --datamodel");
            }
        }
        Some(SubCommand::Cli(cli_command)) => {
            tracing::info!(git_hash = env!("GIT_HASH"), "Starting migration engine CLI");
            cli_command.run(features).await;
        }
    }
}

async fn start_engine(datamodel_location: &str, enabled_preview_features: BitFlags<MigrationFeature>) -> ! {
    use std::io::Read as _;

    tracing::info!(git_hash = env!("GIT_HASH"), "Starting migration engine RPC server",);
    let mut file = std::fs::File::open(datamodel_location).expect("error opening datamodel file");

    let mut datamodel = String::new();
    file.read_to_string(&mut datamodel).unwrap();

    match RpcApi::new(&datamodel, enabled_preview_features).await {
        // Block the thread and handle IO in async until EOF.
        Ok(api) => json_rpc_stdio::run(api.io_handler()).await.unwrap(),
        Err(err) => {
            let (error, exit_code) = match &err {
                CoreError::ReceivedBadDatamodel(message) => {
                    let error = user_facing_errors::UnknownError {
                        message: message.clone(),
                        backtrace: Some(format!("{:?}", user_facing_errors::new_backtrace())),
                    };

                    (user_facing_errors::Error::from(error), 1)
                }
                _ => (err.render_user_facing(), 250),
            };

            serde_json::to_writer(std::io::stdout().lock(), &error).expect("failed to write to stdout");
            std::process::exit(exit_code)
        }
    }

    std::process::exit(0);
}
