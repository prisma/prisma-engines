#[macro_use]
extern crate log;

#[macro_use]
extern crate rust_embed;

mod cli;
mod context;
mod data_model_loader;
mod dmmf;
mod error;
mod exec_loader;
mod request_handlers;
mod server;
mod utilities;

use clap::{App as ClapApp, Arg, SubCommand};
use cli::*;
use error::*;
use lazy_static::lazy_static;
use request_handlers::{PrismaRequest, RequestHandler};
use server::HttpServer;
use std::{env, error::Error, process};
use tracing::subscriber;
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum LogFormat {
    Text,
    Json,
}

lazy_static! {
    pub static ref LOG_FORMAT: LogFormat = {
        match std::env::var("RUST_LOG_FORMAT").as_ref().map(|s| s.as_str()) {
            Ok("devel") => LogFormat::Text,
            _ => LogFormat::Json,
        }
    };
}

pub type PrismaResult<T> = Result<T, PrismaError>;
type AnyError = Box<dyn Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let matches = ClapApp::new("Prisma Query Engine")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("port")
                .help("The port the query engine should bind to.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("legacy")
                .long("legacy")
                .help("Switches query schema generation to Prisma 1 compatible mode.")
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
                    Arg::with_name("dmmf")
                        .long("dmmf")
                        .help("Output the DMMF from the loaded data model.")
                        .takes_value(false)
                        .required(false),
                )
                .arg(
                    Arg::with_name("dmmf_to_dml")
                        .long("dmmf_to_dml")
                        .help("Convert the given DMMF JSON file to a data model")
                        .takes_value(true)
                        .required(false),
                )
                .arg(
                    Arg::with_name("get_config")
                        .long("get_config")
                        .help("Get the configuration from the given data model")
                        .takes_value(true)
                        .required(false),
                ),
        )
        .get_matches();

    if matches.is_present("version") {
        println!(env!("GIT_HASH"));
    } else if let Some(matches) = matches.subcommand_matches("cli") {
        match CliCommand::new(matches) {
            Some(cmd) => {
                if let Err(err) = cmd.execute() {
                    info!("Encountered error during initialization:");
                    err.pretty_print();
                    process::exit(1);
                }
            }
            None => {
                error!("No command provided");
                process::exit(1);
            }
        }
    } else {
        init_logger()?;

        let port = matches
            .value_of("port")
            .map(|p| p.to_owned())
            .or_else(|| env::var("PORT").ok())
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or_else(|| 4466);

        let address = ([0, 0, 0, 0], port);
        let legacy = matches.is_present("legacy");

        if let Err(err) = HttpServer::run(address, legacy).await {
            info!("Encountered error during initialization:");
            err.pretty_print();
            process::exit(1);
        };
    };

    Ok(())
}

fn init_logger() -> Result<(), AnyError> {
    LogTracer::init()?;

    match *LOG_FORMAT {
        LogFormat::Text => {
            let subscriber = FmtSubscriber::builder()
                .with_env_filter(EnvFilter::from_default_env())
                .finish();

            subscriber::set_global_default(subscriber)?;
        }
        LogFormat::Json => {
            let subscriber = FmtSubscriber::builder()
                .json()
                .with_env_filter(EnvFilter::from_default_env())
                .finish();

            subscriber::set_global_default(subscriber)?;

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

    Ok(())
}
