pub mod api;
mod cli;
pub mod commands;
mod error;
pub mod migration;
pub mod migration_engine;

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use crate::api::RpcApi;
use clap::{App, Arg, SubCommand};
use cli::CliError;
use commands::*;
use datamodel::{self, error::ErrorCollection, Datamodel};
use std::{env, fs, io, io::Read};

pub use error::Error;
pub use migration_engine::*;

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn parse_datamodel(datamodel: &str) -> CommandResult<Datamodel> {
    let result = datamodel::parse_datamodel_or_pretty_error(&datamodel, "datamodel file, line");
    result.map_err(|e| CommandError::Generic { code: 1001, error: e })
}

pub(crate) fn pretty_print_errors(errors: ErrorCollection, datamodel: &str) {
    let file_name = env::var("PRISMA_SDL_PATH").unwrap_or_else(|_| "schema.prisma".to_string());

    for error in errors.to_iter() {
        println!();
        error
            .pretty_print(&mut io::stderr().lock(), &file_name, datamodel)
            .expect("Failed to write errors to stderr");
    }
}

fn main() {
    let orig_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        orig_hook(info);
        std::process::exit(255);
    }));

    env_logger::init();

    let matches = App::new("Prisma Migration Engine")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("datamodel_location")
                .short("d")
                .long("datamodel")
                .value_name("FILE")
                .help("Path to the datamodel.")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("single_cmd")
                .short("s")
                .long("single_cmd")
                .help("Run only a single command, then exit")
                .takes_value(false)
                .required(false),
        )
        .arg(
            Arg::with_name("version")
                .long("version")
                .help("Prints the server commit ID")
                .takes_value(false)
                .required(false),
        )
        .subcommand(
            SubCommand::with_name("cli")
                .about("Doesn't start a server, but allows running specific commands against Prisma.")
                .arg(
                    Arg::with_name("datasource")
                        .long("datasource")
                        .short("d")
                        .help("The connection string to the database")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("can_connect_to_database")
                        .long("can_connect_to_database")
                        .help("Does the database connection string work")
                        .takes_value(false)
                        .required(false),
                )
                .arg(
                    Arg::with_name("create_database")
                        .long("create_database")
                        .help("Create an empty database defined in the configuration string.")
                        .takes_value(false)
                        .required(false),
                ),
        )
        .get_matches();

    if matches.is_present("version") {
        println!(env!("GIT_HASH"));
    } else if let Some(matches) = matches.subcommand_matches("cli") {
        let datasource = matches.value_of("datasource").unwrap();

        match cli::run(&matches, &datasource) {
            Ok(msg) => {
                info!("{}", msg);
                std::process::exit(0);
            }
            Err(error) => {
                error!("{}", error);

                match error {
                    CliError::DatabaseDoesNotExist(_) => {
                        std::process::exit(1);
                    }
                    CliError::DatabaseAccessDenied(_) => {
                        std::process::exit(2);
                    }
                    CliError::AuthenticationFailed(_) => {
                        std::process::exit(3);
                    }
                    CliError::ConnectTimeout | CliError::Timeout => {
                        std::process::exit(4);
                    }
                    CliError::DatabaseAlreadyExists(_) => {
                        std::process::exit(5);
                    }
                    CliError::TlsError(_) => {
                        std::process::exit(6);
                    }
                    _ => {
                        std::process::exit(255);
                    }
                }
            }
        }
    } else {
        let dml_loc = matches.value_of("datamodel_location").unwrap();
        let mut file = fs::File::open(&dml_loc).unwrap();

        let mut datamodel = String::new();
        file.read_to_string(&mut datamodel).unwrap();

        if matches.is_present("single_cmd") {
            let api = RpcApi::new_sync(&datamodel).unwrap();
            let response = api.handle().unwrap();

            println!("{}", response);
        } else {
            match RpcApi::new_async(&datamodel) {
                Ok(api) => api.start_server(),
                Err(Error::DatamodelError(errors)) => {
                    pretty_print_errors(errors, &datamodel);
                    std::process::exit(1);
                }
                Err(e) => panic!("{:?}", e),
            }
        }
    }
}
