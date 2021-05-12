#![deny(rust_2018_idioms, unsafe_code)]

mod commands;
mod logger;

use commands::error::CliError;

fn print_help_text() -> ! {
    const HELP_TEXT: &str = r#"
USAGE:
    migration-engine [SUBCOMMAND] [OPTIONS]

SUBCOMMANDS:
    create-database             Create a logical database
    can-connect-to-database     Check that the migration engine can connect to a given database
    drop-database               Drops a logical database
    qe-setup                    Internal setup for query engine tests
    start                       Start the migration engine JSON-RPC server over stdio
"#;

    eprintln!("{}", HELP_TEXT);

    std::process::exit(1);
}

#[tokio::main]
async fn main() {
    logger::init_logger();

    match run_with_args(pico_args::Arguments::from_env()).await {
        Ok(()) => (),
        Err(cli_error) => {
            panic!("CLI error:\n{}", cli_error)
        }
    }
}

async fn run_with_args(mut args: pico_args::Arguments) -> Result<(), CliError> {
    // if args.contains("-h") || args.contains("--help") {
    //     print_help_text()
    // }

    match args.subcommand().expect("Arguments were not UTF-8") {
        None => {
            eprintln!("No subcommand was passed.");
            print_help_text();
        }
        Some(subcommand) => {
            tracing::info!(git_hash = env!("GIT_HASH"), "Starting migration engine CLI");
            commands::run(&subcommand, args).await
        }
    }
}
