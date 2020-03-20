#[macro_use]
extern crate log;
#[macro_use]
extern crate rust_embed;

use cli::*;
use error::*;
use once_cell::sync::Lazy;
use request_handlers::{PrismaRequest, PrismaResponse, RequestHandler};
use server::HttpServer;
use std::{
    convert::TryFrom, error::Error, ffi::OsStr, fs::File, io::Read, net::SocketAddr, process,
};
use structopt::StructOpt;
use tracing::subscriber;
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

mod cli;
mod configuration;
mod context;
mod dmmf;
mod error;
mod exec_loader;
mod request_handlers;
mod server;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum LogFormat {
    Text,
    Json,
}

static LOG_FORMAT: Lazy<LogFormat> = Lazy::new(|| {
    match std::env::var("RUST_LOG_FORMAT")
        .as_ref()
        .map(|s| s.as_str())
    {
        Ok("devel") => LogFormat::Text,
        _ => LogFormat::Json,
    }
});

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
pub struct ExecuteRequestInput {
    /// GraphQL query to execute
    pub query: String,
    /// Run in the legacy GraphQL mode
    #[structopt(long)]
    pub legacy: bool,
}

#[derive(Debug, StructOpt, Clone)]
pub enum CliOpt {
    /// Output the DMMF from the loaded data model.
    Dmmf,
    /// Get the configuration from the given data model.
    GetConfig,
    /// Executes one request and then terminates.
    ExecuteRequest(ExecuteRequestInput),
}

pub fn parse_base64_string(s: &str) -> PrismaResult<String> {
    match base64::decode(s) {
        Ok(bytes) => String::from_utf8(bytes).map_err(|e| {
            trace!("Error decoding {} from Base64 (invalid UTF-8): {:?}", s, e);

            PrismaError::ConfigurationError("Invalid Base64".into())
        }),
        Err(e) => {
            trace!("Decoding Base64 failed (might not be encoded): {:?}", e);
            Ok(String::from(s))
        }
    }
}

pub fn load_datamodel_file(path: &OsStr) -> String {
    let mut f = File::open(path).expect(&format!("Could not open datamodel file {:?}", path));
    let mut datamodel = String::new();

    f.read_to_string(&mut datamodel)
        .expect(&format!("Could not read datamodel file: {:?}", path));

    datamodel
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(version = env!("GIT_HASH"))]
pub struct PrismaOpt {
    /// The hostname or IP the query engine should bind to.
    #[structopt(long, short = "H", default_value = "127.0.0.1")]
    host: String,
    /// The port the query engine should bind to.
    #[structopt(long, short, env, default_value = "4466")]
    port: u16,
    /// Path to the Prisma datamodel file
    #[structopt(long, env = "PRISMA_DML_PATH", parse(from_os_str = load_datamodel_file))]
    datamodel_path: Option<String>,
    /// Base64 encoded Prisma datamodel
    #[structopt(long, env = "PRISMA_DML", parse(try_from_str = parse_base64_string))]
    datamodel: Option<String>,
    /// Base64 encoded datasources, overwriting the ones in the datamodel
    #[structopt(long, env, parse(try_from_str = parse_base64_string))]
    overwrite_datasources: Option<String>,
    /// Switches query schema generation to Prisma 1 compatible mode.
    #[structopt(long, short)]
    legacy: bool,
    /// Runs all queries in a transaction, including all the reads.
    #[structopt(long, short = "t")]
    always_force_transactions: bool,
    /// Enables raw SQL queries with executeRaw mutation
    #[structopt(long, short = "r")]
    enable_raw_queries: bool,
    /// Enables the GraphQL playground
    #[structopt(long, short = "g")]
    enable_playground: bool,
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

            let datamodel = opts
                .datamodel
                .xor(opts.datamodel_path)
                .expect("Datamodel should be provided either as path or base64-encoded string.");

            let builder = HttpServer::builder(datamodel)
                .legacy(opts.legacy)
                .overwrite_datasources(opts.overwrite_datasources)
                .enable_raw_queries(opts.enable_raw_queries)
                .enable_playground(opts.enable_playground)
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
                        tracing::event!(
                            tracing::Level::ERROR,
                            message = "PANIC",
                            reason = payload.as_str()
                        );
                    }
                }

                std::process::exit(255);
            }));
        }
    }

    Ok(())
}
