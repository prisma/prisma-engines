#[cfg(test)]
mod tests;

use crate::CoreResult;
use anyhow::Context;
use clap::ArgMatches;
use itertools::Itertools;
use migration_connector::*;
use quaint::prelude::SqlFamily;
use sql_migration_connector::SqlMigrationConnector;
use std::collections::HashMap;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("Known error: {:?}", error)]
    Known {
        error: user_facing_errors::KnownError,
        exit_code: i32,
    },
    #[error("{}", error)]
    Unknown {
        error: migration_connector::ErrorKind,
        exit_code: i32,
    },

    #[error("No command defined")]
    NoCommandDefined,

    #[error("Unknown error occured: {0}")]
    Other(anyhow::Error),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Known { exit_code, .. } => *exit_code,
            CliError::Unknown { exit_code, .. } => *exit_code,
            _ => 255,
        }
    }

    /// The errors spec error code, if applicable
    #[cfg(test)]
    fn error_code(&self) -> Option<&str> {
        match self {
            CliError::Known {
                error: user_facing_errors::KnownError { error_code, .. },
                ..
            } => Some(error_code),
            _ => None,
        }
    }
}

pub fn exit_code(error_kind: &migration_connector::ErrorKind) -> i32 {
    match error_kind {
        ErrorKind::DatabaseDoesNotExist { .. } => 1,
        ErrorKind::DatabaseAccessDenied { .. } => 2,
        ErrorKind::AuthenticationFailed { .. } => 3,
        ErrorKind::ConnectTimeout | ErrorKind::Timeout => 4,
        ErrorKind::DatabaseAlreadyExists { .. } => 5,
        ErrorKind::TlsError { .. } => 6,
        _ => 255,
    }
}

impl From<ConnectorError> for CliError {
    fn from(e: ConnectorError) -> Self {
        let ConnectorError {
            user_facing_error,
            kind: error_kind,
        } = e;

        let exit_code = exit_code(&error_kind);

        match user_facing_error {
            Some(error) => CliError::Known { error, exit_code },
            None => CliError::Unknown {
                error: error_kind,
                exit_code,
            },
        }
    }
}

impl From<crate::Error> for CliError {
    fn from(e: crate::Error) -> Self {
        match e {
            crate::Error::ConnectorError(e) => e.into(),
            e => Self::Other(e.into()),
        }
    }
}

pub async fn run(matches: &ArgMatches<'_>, datasource: &str) -> Result<String, CliError> {
    if matches.is_present("can_connect_to_database") {
        create_conn(datasource, false).await?;
        Ok("Connection successful".into())
    } else if matches.is_present("create_database") {
        create_database(datasource).await
    } else {
        Err(CliError::NoCommandDefined)
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

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
async fn create_postgres_admin_conn(mut url: Url) -> CoreResult<SqlMigrationConnector> {
    let candidate_default_databases = &["postgres", "template1"];

    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).join("&");
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

pub fn clap_app() -> clap::App<'static, 'static> {
    use clap::{App, Arg, SubCommand};
    App::new("Prisma Migration Engine")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::with_name("datamodel_location")
                .short("d")
                .long("datamodel")
                .value_name("FILE")
                .help("Path to the datamodel.")
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("single_cmd")
                .short("s")
                .long("single_cmd")
                .help("Run only a single command, then exit")
                .takes_value(false)
                .required(false),
        )
        .arg(
            Arg::with_name("version")
                .long("version")
                .help("Prints the server commit ID")
                .takes_value(false)
                .required(false),
        )
        .subcommand(
            SubCommand::with_name("cli")
                .about("Doesn't start a server, but allows running specific commands against Prisma.")
                .arg(
                    Arg::with_name("datasource")
                        .long("datasource")
                        .short("d")
                        .help("The connection string to the database")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("can_connect_to_database")
                        .long("can_connect_to_database")
                        .help("Does the database connection string work")
                        .takes_value(false)
                        .required(false),
                )
                .arg(
                    Arg::with_name("create_database")
                        .long("create_database")
                        .help("Create an empty database defined in the configuration string.")
                        .takes_value(false)
                        .required(false),
                ),
        )
}

pub fn render_error(cli_error: CliError) -> user_facing_errors::Error {
    use user_facing_errors::UnknownError;

    match cli_error {
        CliError::Known { error, .. } => error.into(),
        other => UnknownError {
            message: format!("{}", other),
            backtrace: None,
        }
        .into(),
    }
}

fn split_database_string(database_string: &str) -> Option<(&str, &str)> {
    let mut split = database_string.splitn(2, ':');

    split.next().and_then(|prefix| split.next().map(|rest| (prefix, rest)))
}
