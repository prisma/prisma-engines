pub(crate) mod error;

#[cfg(test)]
mod tests;

use enumflags2::BitFlags;
use error::CliError;
use migration_core::{migration_api, qe_setup::QueryEngineFlags};
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
    #[structopt(long, short = "f", parse(try_from_str = parse_setup_flags))]
    qe_test_setup_flags: Option<BitFlags<QueryEngineFlags>>,
    #[structopt(subcommand)]
    command: CliCommand,
}

impl Cli {
    pub(crate) async fn run(self) -> ! {
        match self.run_inner().await {
            Ok(msg) => {
                tracing::info!("{}", msg);
                std::process::exit(0);
            }
            Err(error) => {
                tracing::error!(
                    is_panic = false,
                    error_code = error.error_code().unwrap_or(""),
                    "{}",
                    error
                );
                let exit_code = error.exit_code();

                std::process::exit(exit_code)
            }
        }
    }

    pub(crate) async fn run_inner(self) -> Result<String, CliError> {
        match self.command {
            CliCommand::CreateDatabase => create_database(&self.datasource).await,
            CliCommand::CanConnectToDatabase => connect_to_database(&self.datasource).await,
            CliCommand::DropDatabase => drop_database(&self.datasource).await,
            CliCommand::QeSetup => {
                qe_setup(
                    &self.datasource,
                    self.qe_test_setup_flags.unwrap_or_else(BitFlags::empty),
                )
                .await?;
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

fn parse_setup_flags(s: &str) -> Result<BitFlags<QueryEngineFlags>, CliError> {
    let mut flags = BitFlags::empty();

    for flag in s.split(',') {
        match flag {
            "database_creation_not_allowed" => flags.insert(QueryEngineFlags::DatabaseCreationNotAllowed),
            "" => (),
            flag => return Err(CliError::invalid_parameters(format!("Unknown flag: {}", flag))),
        }
    }

    Ok(flags)
}

async fn connect_to_database(database_str: &str) -> Result<String, CliError> {
    let datamodel = datasource_from_database_str(database_str)?;
    migration_api(&datamodel).await?;
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

    Ok("The database was successfully dropped.".to_string())
}

async fn qe_setup(prisma_schema: &str, flags: BitFlags<QueryEngineFlags>) -> Result<(), CliError> {
    migration_core::qe_setup::run(&prisma_schema, flags).await?;

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
