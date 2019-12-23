pub mod api;
pub mod cli;
pub mod commands;
mod error;
pub mod migration;
pub mod migration_engine;

use crate::api::RpcApi;
use commands::*;
use datamodel::{self, Datamodel};
use futures::FutureExt;
use std::{fs, io::Read};
#[cfg(test)]
mod tests;

pub use error::Error;
pub use migration_engine::*;

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn parse_datamodel(datamodel: &str) -> CommandResult<Datamodel> {
    let result = datamodel::parse_datamodel_or_pretty_error(&datamodel, "datamodel file, line");
    result.map_err(|e| CommandError::DataModelErrors { errors: vec![e] })
}

#[tokio::main]
async fn main() {
    user_facing_errors::set_panic_hook();
    init_logger();
    let mut global_exit_code: i32 = 0;

    let matches = cli::clap_app().get_matches();

    if matches.is_present("version") {
        println!(env!("GIT_HASH"));
    } else if let Some(matches) = matches.subcommand_matches("cli") {
        tracing::info!(git_hash = env!("GIT_HASH"), "Starting migration engine CLI");
        let datasource = matches.value_of("datasource").unwrap();

        match std::panic::AssertUnwindSafe(cli::run(&matches, &datasource))
            .catch_unwind()
            .await
        {
            Ok(Ok(msg)) => {
                tracing::info!("{}", msg);
                global_exit_code = 0;
            }
            Ok(Err(error)) => {
                tracing::error!("{}", error);
                let exit_code = error.exit_code();
                serde_json::to_writer(std::io::stdout(), &cli::render_error(error)).expect("failed to write to stdout");
                println!();
                global_exit_code = exit_code;
            }
            Err(panic) => {
                serde_json::to_writer(
                    std::io::stdout(),
                    &user_facing_errors::Error::from_panic_payload(panic.as_ref()),
                )
                .expect("failed to write to stdout");
                println!();
                global_exit_code = 255;
            }
        }
    } else {
        tracing::info!(git_hash = env!("GIT_HASH"), "Starting migration engine RPC server",);
        let dml_loc = matches.value_of("datamodel_location").unwrap();
        let mut file = fs::File::open(&dml_loc).unwrap();

        let mut datamodel = String::new();
        file.read_to_string(&mut datamodel).unwrap();

        if matches.is_present("single_cmd") {
            let api = RpcApi::new(&datamodel).await.unwrap();
            let response = api.handle().unwrap();

            println!("{}", response);
        } else {
            match RpcApi::new(&datamodel).await {
                Ok(api) => api.start_server().await,
                Err(err) => {
                    let (error, exit_code) = match &err {
                        Error::DatamodelError(errors) => {
                            let error = user_facing_errors::UnknownError {
                                message: api::pretty_print_datamodel_errors(errors, &datamodel)
                                    .expect("rendering error"),
                                backtrace: Some(format!("{:?}", user_facing_errors::new_backtrace())),
                            };

                            (user_facing_errors::Error::from(error), 1)
                        }
                        _ => (api::render_error(err), 255),
                    };

                    serde_json::to_writer(std::io::stdout().lock(), &error).expect("failed to write to stdout");
                    global_exit_code = exit_code
                }
            }
        }
    }

    std::process::exit(global_exit_code);
}

fn init_logger() {
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .init()
}
