#[macro_use]
extern crate log;
#[macro_use]
extern crate rust_embed;

use std::{convert::TryFrom, error::Error, net::SocketAddr, process};

use structopt::StructOpt;
use tracing::subscriber;
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use cli::*;
use error::*;
use lazy_static::lazy_static;
use request_handlers::{PrismaRequest, PrismaResponse, RequestHandler};
use server::HttpServer;

mod cli;
mod context;
mod data_model_loader;
mod dmmf;
mod error;
mod exec_loader;
mod request_handlers;
mod server;
#[cfg(test)]
mod tests;
mod utilities;

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

#[derive(Debug, StructOpt, Clone)]
pub enum Subcommand {
    /// Doesn't start a server, but allows running specific commands against Prisma.
    Cli(CliOpt),
}

#[derive(Debug, Clone, StructOpt)]
pub struct DmmfToDmlInput {
    #[structopt(name = "path")]
    pub path: String,
}

#[derive(Debug, Clone, StructOpt)]
pub struct GetConfigInput {
    pub path: String,
}

#[derive(Debug, Clone, StructOpt)]
pub struct ExecuteRequestInput {
    pub query: String,
}

#[derive(Debug, StructOpt, Clone)]
pub enum CliOpt {
    /// Output the DMMF from the loaded data model.
    #[structopt(name = "--dmmf")]
    Dmmf,
    /// Convert the given DMMF JSON file to a data model.
    #[structopt(name = "--dmmf_to_dml")]
    DmmfToDml(DmmfToDmlInput),
    /// Get the configuration from the given data model.
    #[structopt(name = "--get_config")]
    GetConfig(GetConfigInput),
    /// Executes one request and then terminates.
    #[structopt(name = "--execute_request")]
    ExecuteRequest(ExecuteRequestInput),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(version = env!("GIT_HASH"))]
pub struct PrismaOpt {
    /// The hostname or IP the query engine should bind to.
    #[structopt(long, default_value = "127.0.0.1")]
    host: String,
    /// The port the query engine should bind to.
    #[structopt(long, short, env = "PORT", default_value = "4466")]
    port: u16,
    /// Switches query schema generation to Prisma 1 compatible mode.
    #[structopt(long)]
    legacy: bool,
    /// Runs all queries in a transaction, including all the reads.
    #[structopt(long = "always_force_transactions")]
    always_force_transactions: bool,
    /// Enables raw SQL queries with executeRaw mutation
    #[structopt(long = "enable_raw_queries")]
    enable_raw_queries: bool,
    #[structopt(subcommand)]
    subcommand: Option<Subcommand>,
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    init_logger()?;
    let opts = PrismaOpt::from_args();

    match CliCommand::try_from(&opts) {
        Ok(cmd) => {
            if let Err(err) = cmd.execute().await {
                info!("Encountered error during initialization:");
                err.render_as_json().expect("error rendering");
                process::exit(1);
            }
        }
        Err(_) => {
            set_panic_hook()?;
            let ip = opts.host.parse().expect("Host was not a valid IP address");
            let address = SocketAddr::new(ip, opts.port);

            eprintln!("Printing to stderr for debugging");
            eprintln!("Listening on {}:{}", opts.host, opts.port);

            let builder = HttpServer::builder()
                .legacy(opts.legacy)
                .enable_raw_queries(opts.enable_raw_queries)
                .force_transactions(opts.always_force_transactions);

            if let Err(err) = builder.build_and_run(address).await {
                info!("Encountered error during initialization:");
                err.render_as_json().expect("error rendering");
                process::exit(1);
            };
        }
    }

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
        }
    }

    Ok(())
}

fn set_panic_hook() -> Result<(), AnyError> {
    match *LOG_FORMAT {
        LogFormat::Text => (),
        LogFormat::Json => {
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
