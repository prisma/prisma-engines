#[macro_use]
extern crate log;
#[macro_use]
extern crate rust_embed;

use prisma::{
    AnyError, set_panic_hook, init_logger, server::{HttpServerBuilder, HttpServer},
    error::PrismaError, cli::CliCommand, PrismaResult, opt::PrismaOpt,
};
use std::process;
use structopt::StructOpt;
use std::net::SocketAddr;
use std::convert::TryFrom;

#[cfg(test)]
mod tests;

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
        Err(PrismaError::InvocationError(_)) => {
            set_panic_hook()?;
            let ip = opts.host.parse().expect("Host was not a valid IP address");
            let address = SocketAddr::new(ip, opts.port);

            eprintln!("Printing to stderr for debugging");
            eprintln!("Listening on {}:{}", opts.host, opts.port);

            let create_builder = move || {
                let config = opts.configuration(false)?;
                let datamodel = opts.datamodel(false)?;

                PrismaResult::<HttpServerBuilder>::Ok(
                    HttpServer::builder(config, datamodel)
                        .legacy(opts.legacy)
                        .enable_raw_queries(opts.enable_raw_queries)
                        .enable_playground(opts.enable_playground)
                        .force_transactions(opts.always_force_transactions),
                )
            };

            let builder = match create_builder() {
                Err(err) => {
                    info!("Encountered error during initialization:");
                    err.render_as_json().expect("error rendering");
                    process::exit(1);
                }
                Ok(builder) => builder,
            };

            if let Err(err) = builder.build_and_run(address).await {
                info!("Encountered error during initialization:");
                err.render_as_json().expect("error rendering");
                process::exit(1);
            };
        }
        Err(err) => {
            info!("Encountered error during initialization:");
            err.render_as_json().expect("error rendering");
            process::exit(1);
        }
    }

    Ok(())
}
