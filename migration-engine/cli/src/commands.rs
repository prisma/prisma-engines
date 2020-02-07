pub(crate) mod error;
#[cfg(test)]
mod tests;

use anyhow::Context;
use error::CliError;
use futures::FutureExt;
use migration_connector::{ConnectorError, ErrorKind, MigrationConnector};
use migration_core::CoreResult;
use quaint::prelude::SqlFamily;
use sql_migration_connector::SqlMigrationConnector;
use std::collections::HashMap;
use structopt::StructOpt;
use url::Url;

#[derive(Debug, StructOpt)]
pub(crate) struct Cli {
    /// The connection string to the database
    #[structopt(long = "datasource", short = "d")]
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
            CliCommand::CanConnectToDatabase => {
                create_conn(&self.datasource, false).await?;
                Ok("Connection successful".to_owned())
            }
        }
    }
}

#[derive(Debug, StructOpt)]
enum CliCommand {
    /// Create an empty database defined in the configuration string.
    #[structopt(name = "--create_database")]
    CreateDatabase,
    /// Does the database connection string work?
    #[structopt(name = "--can_connect_to_database")]
    CanConnectToDatabase,
}

async fn create_conn(datasource: &str, admin_mode: bool) -> CoreResult<(String, Box<SqlMigrationConnector>)> {
    let mut url = Url::parse(datasource).expect("Invalid url in the datasource");
    let sql_family = SqlFamily::from_scheme(url.scheme());

    match sql_family {
        Some(SqlFamily::Sqlite) => {
            let connector = SqlMigrationConnector::new(datasource, "sqlite").await?;
            Ok((datasource.to_owned(), Box::new(connector)))
        }
        Some(SqlFamily::Postgres) => {
            let db_name = fetch_db_name(&url, "postgres");

            let connector = if admin_mode {
                create_postgres_admin_conn(url).await?
            } else {
                SqlMigrationConnector::new(url.as_str(), "postgres").await?
            };

            Ok((db_name, Box::new(connector)))
        }
        Some(SqlFamily::Mysql) => {
            let db_name = fetch_db_name(&url, "mysql");

            if admin_mode {
                url.set_path("");
            }

            let inner = SqlMigrationConnector::new(url.as_str(), "mysql").await?;
            Ok((db_name, Box::new(inner)))
        }
        None => unimplemented!("Connector {} is not supported yet", url.scheme()),
    }
}

fn fetch_db_name(url: &Url, default: &str) -> String {
    let result = match url.path_segments() {
        Some(mut segments) => segments.next().unwrap_or(default),
        None => default,
    };

    String::from(result)
}

async fn create_database(datasource: &str) -> Result<String, CliError> {
    let url = split_database_string(datasource)
        .and_then(|(prefix, rest)| SqlFamily::from_scheme(prefix).map(|family| (family, rest)));

    match url {
        Some((SqlFamily::Sqlite, path)) => {
            let path = std::path::Path::new(path);
            if path.exists() {
                return Ok(String::new());
            }

            let dir = path.parent();

            if let Some((dir, false)) = dir.map(|dir| (dir, dir.exists())) {
                std::fs::create_dir_all(dir)
                    .context("Creating SQLite database parent directory.")
                    .map_err(|io_err| CliError::Other(io_err.into()))?;
            }

            create_conn(datasource, true).await?;

            Ok(String::new())
        }
        Some(_) => {
            let (db_name, conn) = create_conn(datasource, true).await?;
            conn.create_database(&db_name).await?;
            Ok(format!("Database '{}' created successfully.", db_name))
        }
        None => Err(CliError::Other(anyhow::anyhow!(
            "Invalid URL or unsupported connector in the datasource ({:?})",
            url
        ))),
    }
}

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
async fn create_postgres_admin_conn(mut url: Url) -> CoreResult<SqlMigrationConnector> {
    let candidate_default_databases = &["postgres", "template1"];

    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params: Vec<String> = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let params: String = params.join("&");
    url.set_query(Some(&params));

    let mut connector = None;

    for database_name in candidate_default_databases {
        url.set_path(&format!("/{}", database_name));
        match SqlMigrationConnector::new(url.as_str(), "postgresql").await {
            // If the database does not exist, try the next one.
            Err(err) => match &err.kind {
                migration_connector::ErrorKind::DatabaseDoesNotExist { .. } => (),
                _other_outcome => {
                    connector = Some(Err(err));
                    break;
                }
            },
            // If the outcome is anything else, use this.
            other_outcome => {
                connector = Some(other_outcome);
                break;
            }
        }
    }

    let connector = connector
        .ok_or_else(|| {
            ConnectorError::from_kind(ErrorKind::DatabaseCreationFailed {
                explanation: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned()
            })
        })??;

    Ok(connector)
}

fn split_database_string(database_string: &str) -> Option<(&str, &str)> {
    let mut split = database_string.splitn(2, ':');

    split.next().and_then(|prefix| split.next().map(|rest| (prefix, rest)))
}
