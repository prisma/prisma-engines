pub(crate) mod error;
#[cfg(test)]
mod tests;

use error::CliError;
use futures::FutureExt;
use sql_migration_connector::SqlMigrationConnector;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub(crate) struct Cli {
    /// The connection string to the database
    #[structopt(long, short = "d")]
    datasource: String,
    #[structopt(subcommand)]
    command: CliCommand,
}

impl Cli {
    pub(crate) async fn run(&self) -> ! {
        match std::panic::AssertUnwindSafe(self.run_inner()).catch_unwind().await {
            Ok(Ok(msg)) => {
                tracing::info!("{}", msg);
                std::process::exit(0);
            }
            Ok(Err(error)) => {
                tracing::error!("{}", error);
                let exit_code = error.exit_code();
                serde_json::to_writer(std::io::stdout(), &error::render_error(error))
                    .expect("failed to write to stdout");
                println!();
                std::process::exit(exit_code)
            }
            Err(panic) => {
                serde_json::to_writer(
                    std::io::stdout(),
                    &user_facing_errors::Error::from_panic_payload(panic.as_ref()),
                )
                .expect("failed to write to stdout");
                println!();
                std::process::exit(255);
            }
        }
    }

    pub(crate) async fn run_inner(&self) -> Result<String, CliError> {
        match self.command {
            CliCommand::CreateDatabase => create_database(&self.datasource).await,
            CliCommand::CanConnectToDatabase => connect_to_database(&self.datasource).await,
        }
    }
}

#[derive(Debug, StructOpt)]
enum CliCommand {
    /// Create an empty database defined in the configuration string.
    CreateDatabase,
    /// Does the database connection string work?
    CanConnectToDatabase,
}

async fn connect_to_database(database_str: &str) -> Result<String, CliError> {
    SqlMigrationConnector::new(database_str).await?;
    Ok("Connection successful".to_owned())
}

async fn create_database(datasource: &str) -> Result<String, CliError> {
    let db_name = SqlMigrationConnector::create_database(datasource).await?;
    Ok(format!("Database '{}' was successfully created.", db_name))
}
