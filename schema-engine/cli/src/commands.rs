use crate::logger::log_error_and_exit;
use base64::prelude::*;
use schema_connector::ConnectorError;
use schema_core::json_rpc::types::{DatasourceParam, UrlContainer};
use structopt::StructOpt;
use user_facing_errors::common::SchemaParserError;

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
        let api = schema_core::schema_api(None, None)?;
        match self.command {
            CliCommand::CreateDatabase => {
                let schema_core::json_rpc::types::CreateDatabaseResult { database_name } = api
                    .create_database(schema_core::json_rpc::types::CreateDatabaseParams {
                        datasource: DatasourceParam::ConnectionString(UrlContainer {
                            url: self.datasource.clone(),
                        }),
                    })
                    .await?;
                Ok(format!("Database '{database_name}' was successfully created."))
            }
            CliCommand::CanConnectToDatabase => {
                api.ensure_connection_validity(schema_core::json_rpc::types::EnsureConnectionValidityParams {
                    datasource: DatasourceParam::ConnectionString(UrlContainer {
                        url: self.datasource.clone(),
                    }),
                })
                .await?;
                Ok("Connection successful".to_owned())
            }
            CliCommand::DropDatabase => {
                api.drop_database(self.datasource.clone()).await?;
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
    match BASE64_STANDARD.decode(s) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(e) => Err(ConnectorError::user_facing(SchemaParserError {
                full_error: e.to_string(),
            })),
        },
        Err(_) => Ok(String::from(s)),
    }
}
