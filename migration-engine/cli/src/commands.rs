use crate::logger::log_error_and_exit;
use migration_connector::ConnectorError;
use migration_core::migration_api;
use structopt::StructOpt;
use user_facing_errors::common::{InvalidConnectionString, SchemaParserError};

#[derive(Debug, StructOpt)]
pub(crate) struct Cli {
    /// The connection string to the database
    #[structopt(long, short = "d", parse(try_from_str = parse_base64_string))]
    datasource: String,
    #[structopt(subcommand)]
    command: CliCommand,
}

impl Cli {
    pub(crate) async fn run(self) {
        match self.run_inner().await {
            Ok(msg) => {
                tracing::info!("{}", msg);
            }
            Err(error) => log_error_and_exit(error),
        }
    }

    pub(crate) async fn run_inner(self) -> Result<String, ConnectorError> {
        let datamodel = datasource_from_database_str(&self.datasource)?;
        let api = migration_api(&datamodel).await?;
        match self.command {
            CliCommand::CreateDatabase => {
                let db_name = api.create_database().await?;
                Ok(format!("Database '{}' was successfully created.", db_name))
            }
            CliCommand::CanConnectToDatabase => {
                api.ensure_connection_validity().await?;
                Ok("Connection successful".to_owned())
            }
            CliCommand::DropDatabase => {
                api.drop_database().await?;
                Ok("The database was successfully dropped.".to_owned())
            }
        }
    }
}

#[derive(Debug, StructOpt)]
#[allow(clippy::enum_variant_names)] // disagee
enum CliCommand {
    /// Create an empty database defined in the configuration string.
    CreateDatabase,
    /// Does the database connection string work?
    CanConnectToDatabase,
    /// Drop the database.
    DropDatabase,
}

fn parse_base64_string(s: &str) -> Result<String, ConnectorError> {
    match base64::decode(s) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(e) => Err(ConnectorError::user_facing(SchemaParserError {
                full_error: e.to_string(),
            })),
        },
        Err(_) => Ok(String::from(s)),
    }
}

fn datasource_from_database_str(database_str: &str) -> Result<String, ConnectorError> {
    let provider = match database_str.split(':').next() {
        Some("postgres") => "postgresql",
        Some("file") => "sqlite",
        Some("mongodb+srv") => "mongodb",
        Some(other) => other,
        None => {
            return Err(ConnectorError::user_facing(InvalidConnectionString {
                details: String::new(),
            }))
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
