pub(crate) mod error;
#[cfg(test)]
mod tests;

use error::CliError;
use futures::FutureExt;
use migration_core::migration_api;
use structopt::StructOpt;
use user_facing_errors::{
    common::{InvalidDatabaseString, SchemaParserError},
    KnownError,
};

#[derive(Debug, StructOpt)]
pub(crate) struct Cli {
    /// The connection string to the database
    #[structopt(long, short = "d", parse(try_from_str = parse_base64_string))]
    datasource: String,
    #[structopt(subcommand)]
    command: CliCommand,
}

impl Cli {
    pub(crate) async fn run(self, enabled_preview_features: Vec<String>) -> ! {
        match std::panic::AssertUnwindSafe(self.run_inner(enabled_preview_features))
            .catch_unwind()
            .await
        {
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

    pub(crate) async fn run_inner(self, enabled_preview_features: Vec<String>) -> Result<String, CliError> {
        match self.command {
            CliCommand::CreateDatabase => create_database(&self.datasource).await,
            CliCommand::CanConnectToDatabase => connect_to_database(&self.datasource, enabled_preview_features).await,
            CliCommand::DropDatabase => drop_database(&self.datasource).await,
            CliCommand::QeSetup => {
                qe_setup(&self.datasource).await?;
                Ok(String::new())
            }
        }
    }
}

#[derive(Debug, StructOpt)]
enum CliCommand {
    /// Create an empty database defined in the configuration string.
    CreateDatabase,
    /// Does the database connection string work?
    CanConnectToDatabase,
    /// Drop the database.
    DropDatabase,
    /// Set up the database for connector-test-kit.
    QeSetup,
}

fn parse_base64_string(s: &str) -> Result<String, CliError> {
    match base64::decode(s) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(e) => Err(CliError::Known {
                error: KnownError::new(SchemaParserError {
                    full_error: format!("{}", e),
                }),
                exit_code: 255,
            }),
        },
        Err(_) => Ok(String::from(s)),
    }
}

async fn connect_to_database(database_str: &str, enabled_preview_features: Vec<String>) -> Result<String, CliError> {
    let datamodel = datasource_from_database_str(database_str)?;
    migration_api(&datamodel, enabled_preview_features).await?;
    Ok("Connection successful".to_owned())
}

async fn create_database(database_str: &str) -> Result<String, CliError> {
    let datamodel = datasource_from_database_str(database_str)?;
    let db_name = migration_core::create_database(&datamodel).await?;

    Ok(format!("Database '{}' was successfully created.", db_name))
}

async fn drop_database(database_str: &str) -> Result<String, CliError> {
    let datamodel = datasource_from_database_str(database_str)?;
    migration_core::drop_database(&datamodel).await?;

    Ok(format!("The database was successfully dropped."))
}

async fn qe_setup(prisma_schema: &str) -> Result<(), CliError> {
    migration_core::qe_setup(&prisma_schema).await?;

    Ok(())
}

fn datasource_from_database_str(database_str: &str) -> Result<String, CliError> {
    let provider = match database_str.split(':').next() {
        Some("postgres") => "postgresql",
        Some("file") => "sqlite",
        Some(other) => other,
        None => {
            return Err(CliError::Known {
                error: KnownError::new(InvalidDatabaseString { details: String::new() }),
                exit_code: 255,
            })
        }
    };

    let schema = format!(
        r#"
            datasource db {{
                provider = "{provider}"
                url = "{url}"
            }}
        "#,
        provider = provider,
        url = database_str,
    );

    Ok(schema)
}
