use clap::ArgMatches;
use failure::Fail;
use itertools::Itertools;
use migration_connector::*;
use quaint::prelude::SqlFamily;
use sql_migration_connector::SqlMigrationConnector;
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Fail, PartialEq)]
pub enum CliError {
    #[fail(display = "Database '{}' does not exist.", _0)]
    DatabaseDoesNotExist(String),
    #[fail(display = "Access denied to database '{}'", _0)]
    DatabaseAccessDenied {
        database_name: String,
        database_user: String,
    },
    #[fail(display = "Authentication failed for user '{}'", _0)]
    AuthenticationFailed(String),
    #[fail(display = "Database '{}' already exists", _0)]
    DatabaseAlreadyExists {
        database_name: String,
        database_host: String,
        database_port: u16,
    },
    #[fail(display = "Error connecting to the database")]
    ConnectionError {
        database_host: String,
        database_port: String,
    },
    #[fail(display = "No command defined")]
    NoCommandDefined,
    #[fail(display = "Connect timed out")]
    ConnectTimeout,
    #[fail(display = "Operation timed out")]
    Timeout,
    #[fail(display = "Error opening a TLS connection. {}", _0)]
    TlsError(String),
    #[fail(display = "Unknown error occured: {}", _0)]
    Other(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::DatabaseDoesNotExist(_) => 1,
            CliError::DatabaseAccessDenied { .. } => 2,
            CliError::AuthenticationFailed(_) => 3,
            CliError::ConnectTimeout | CliError::Timeout => 4,
            CliError::DatabaseAlreadyExists { .. } => 5,
            CliError::TlsError(_) => 6,
            _ => 255,
        }
    }
}

impl From<ConnectorError> for CliError {
    fn from(e: ConnectorError) -> Self {
        match e {
            ConnectorError::DatabaseDoesNotExist { db_name, .. } => Self::DatabaseDoesNotExist(db_name),
            ConnectorError::DatabaseAccessDenied {
                database_name,
                database_user,
            } => Self::DatabaseAccessDenied {
                database_name,
                database_user,
            },
            ConnectorError::DatabaseAlreadyExists {
                db_name,
                database_host,
                database_port,
            } => CliError::DatabaseAlreadyExists {
                database_name: db_name,
                database_host,
                database_port,
            },
            ConnectorError::AuthenticationFailed { user, .. } => CliError::AuthenticationFailed(user),
            ConnectorError::ConnectTimeout => CliError::ConnectTimeout,
            ConnectorError::Timeout => CliError::Timeout,
            ConnectorError::TlsError { message } => CliError::TlsError(message),
            ConnectorError::ConnectionError { host, port, cause: _ } => CliError::ConnectionError {
                database_host: host.clone(),
                database_port: port.map(|p| format!("{}", p)).unwrap_or_else(|| "<port>".to_owned()),
            },
            other => CliError::Other(format!("{}", other)),
        }
    }
}

impl From<crate::Error> for CliError {
    fn from(e: crate::Error) -> Self {
        match e {
            crate::Error::ConnectorError(e) => e.into(),
            e => Self::Other(format!("{}", e)),
        }
    }
}

pub async fn run(matches: &ArgMatches<'_>, datasource: &str) -> std::result::Result<String, CliError> {
    if matches.is_present("can_connect_to_database") {
        create_conn(datasource, false).await?;
        Ok("Connection successful".into())
    } else if matches.is_present("create_database") {
        let (db_name, conn) = create_conn(datasource, true).await?;
        conn.create_database(&db_name).await?;
        Ok(format!("Database '{}' created successfully.", db_name))
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

async fn create_conn(datasource: &str, admin_mode: bool) -> crate::Result<(String, Box<SqlMigrationConnector>)> {
    let mut url = Url::parse(datasource).expect("Invalid url in the datasource");
    let sql_family = SqlFamily::from_scheme(url.scheme());

    match sql_family {
        Some(SqlFamily::Sqlite) => {
            let inner = SqlMigrationConnector::new(datasource).await?;

            Ok((String::new(), Box::new(inner)))
        }
        Some(SqlFamily::Postgres) => {
            let db_name = fetch_db_name(&url, "postgres");

            let connector = if admin_mode {
                create_postgres_admin_conn(url).await?
            } else {
                SqlMigrationConnector::new(url.as_str()).await?
            };

            Ok((db_name, Box::new(connector)))
        }
        Some(SqlFamily::Mysql) => {
            let db_name = fetch_db_name(&url, "mysql");

            if admin_mode {
                url.set_path("");
            }

            let inner = SqlMigrationConnector::new(url.as_str()).await?;
            Ok((db_name, Box::new(inner)))
        }
        None => unimplemented!("Connector {} is not supported yet", url.scheme()),
    }
}

/// Try to connect as an admin to a postgres database. We try to pick a default database from which
/// we can create another database.
async fn create_postgres_admin_conn(mut url: Url) -> crate::Result<SqlMigrationConnector> {
    let candidate_default_databases = &["postgres", "template1"];

    let mut params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    params.remove("schema");
    let params = params.into_iter().map(|(k, v)| format!("{}={}", k, v)).join("&");
    url.set_query(Some(&params));

    let mut connector = None;

    for database_name in candidate_default_databases {
        url.set_path(database_name);
        match SqlMigrationConnector::new(url.as_str()).await {
            // If the database does not exist, try the next one.
            Err(migration_connector::ConnectorError::DatabaseDoesNotExist { .. }) => (),
            // If the outcome is anything else, use this.
            other_outcome => {
                connector = Some(other_outcome);
                break;
            }
        }
    }

    let connector = connector
        .ok_or_else(|| {
            ConnectorError::DatabaseCreationFailed {
                explanation: "Prisma could not connect to a default database (`postgres` or `template1`), it cannot create the specified database.".to_owned()
            }
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
    use user_facing_errors::{Error, KnownError, UnknownError};

    match cli_error {
        CliError::DatabaseAlreadyExists {
            database_name,
            database_host,
            database_port,
        } => KnownError::new(user_facing_errors::common::DatabaseAlreadyExists {
            database_host,
            database_name,
            database_port,
        })
        .map(Error::Known)
        .unwrap(),
        CliError::DatabaseAccessDenied {
            database_name,
            database_user,
        } => KnownError::new(user_facing_errors::common::DatabaseAccessDenied {
            database_name,
            database_user,
        })
        .map(Error::Known)
        .unwrap(),
        CliError::ConnectionError {
            database_host: host,
            database_port: port,
        } => KnownError::new(user_facing_errors::common::DatabaseNotReachable {
            database_host: host.clone(),
            database_port: port,
        })
        .map(Error::Known)
        .unwrap(),
        _ => UnknownError {
            message: cli_error.to_string(),
            backtrace: None,
        }
        .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::CliError;
    use clap::ArgMatches;
    use once_cell::sync::Lazy;
    use quaint::prelude::*;

    static TEST_ASYNC_RUNTIME: Lazy<tokio::runtime::Runtime> =
        Lazy::new(|| tokio::runtime::Runtime::new().expect("failed to start tokio test runtime"));

    fn run_sync(matches: &ArgMatches<'_>, datasource: &str) -> Result<String, CliError> {
        TEST_ASYNC_RUNTIME.block_on(super::run(matches, datasource))
    }

    async fn run(args: &[&str], datasource: &str) -> Result<String, CliError> {
        let mut complete_args = vec!["me", "cli", "--datasource", datasource];
        complete_args.extend(args);
        let matches = super::clap_app().get_matches_from(complete_args);
        super::run(&matches.subcommand_matches("cli").unwrap(), datasource).await
    }

    fn with_cli<F>(matches: Vec<&str>, f: F) -> Result<(), Box<dyn std::any::Any + Send + 'static>>
    where
        F: FnOnce(&clap::ArgMatches) -> () + std::panic::UnwindSafe,
    {
        let matches = clap::App::new("cli")
            .arg(
                clap::Arg::with_name("can_connect_to_database")
                    .long("can_connect_to_database")
                    .takes_value(false)
                    .required(false),
            )
            .arg(
                clap::Arg::with_name("create_database")
                    .long("create_database")
                    .help("Create an empty database defined in the configuration string.")
                    .takes_value(false)
                    .required(false),
            )
            .get_matches_from(matches);

        std::panic::catch_unwind(|| f(&matches))
    }

    fn postgres_url(db: Option<&str>) -> String {
        postgres_url_with_scheme(db, "postgresql")
    }

    fn postgres_url_with_scheme(db: Option<&str>, scheme: &str) -> String {
        match std::env::var("IS_BUILDKITE") {
            Ok(_) => format!(
                "{scheme}://postgres:prisma@test-db-postgres:5432/{db_name}",
                scheme = scheme,
                db_name = db.unwrap_or("postgres")
            ),
            _ => format!(
                "{scheme}://postgres:prisma@127.0.0.1:5432/{db_name}?schema=migration-engine",
                scheme = scheme,
                db_name = db.unwrap_or("postgres")
            ),
        }
    }

    fn mysql_url(db: Option<&str>) -> String {
        match std::env::var("IS_BUILDKITE") {
            Ok(_) => format!("mysql://root:prisma@test-db-mysql-5-7:3306/{}", db.unwrap_or("")),
            _ => format!("mysql://root:prisma@127.0.0.1:3306/{}", db.unwrap_or("")),
        }
    }

    #[test]
    fn test_with_missing_command() {
        with_cli(vec!["cli"], |matches| {
            assert_eq!(Err(CliError::NoCommandDefined), run_sync(&matches, &mysql_url(None)));
        })
        .unwrap();
    }

    #[test]
    fn test_connecting_with_a_working_mysql_connection_string() {
        with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
            assert_eq!(
                Ok(String::from("Connection successful")),
                run_sync(&matches, &mysql_url(None))
            );
        })
        .unwrap();
    }

    #[test]
    fn test_connecting_with_a_non_working_mysql_connection_string() {
        let dm = mysql_url(Some("this_does_not_exist"));

        with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
            assert_eq!(
                Err(CliError::DatabaseDoesNotExist(String::from("this_does_not_exist"))),
                run_sync(&matches, &dm)
            );
        })
        .unwrap();
    }

    #[test]
    fn test_connecting_with_a_working_psql_connection_string() {
        with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
            assert_eq!(
                Ok(String::from("Connection successful")),
                run_sync(&matches, &postgres_url(None))
            );
        })
        .unwrap();
    }

    #[test]
    fn test_connecting_with_a_working_psql_connection_string_with_postgres_scheme() {
        with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
            assert_eq!(
                Ok(String::from("Connection successful")),
                run_sync(&matches, &postgres_url_with_scheme(None, "postgres"))
            );
        })
        .unwrap();
    }

    #[test]
    fn test_connecting_with_a_non_working_psql_connection_string() {
        let dm = postgres_url(Some("this_does_not_exist"));

        with_cli(vec!["cli", "--can_connect_to_database"], |matches| {
            assert_eq!(
                Err(CliError::DatabaseDoesNotExist(String::from("this_does_not_exist"))),
                run_sync(&matches, &dm)
            );
        })
        .unwrap();
    }

    #[tokio::test]
    async fn test_create_mysql_database() {
        let url = mysql_url(Some("this_should_exist"));

        let res = run(&["--create_database"], &url).await;

        assert_eq!(
            Ok(String::from("Database 'this_should_exist' created successfully.")),
            res
        );

        if let Ok(_) = res {
            let res = run(&["--can_connect_to_database"], &url).await;
            assert_eq!(Ok(String::from("Connection successful")), res);

            {
                let uri = mysql_url(None);
                let conn = Quaint::new(&uri).unwrap();

                conn.execute_raw("DROP DATABASE `this_should_exist`", &[])
                    .await
                    .unwrap();
            }

            res.unwrap();
        } else {
            res.unwrap();
        }
    }

    #[tokio::test]
    async fn test_create_psql_database() {
        let url = postgres_url(Some("this_should_exist"));

        let res = run(&["--create_database"], &url).await;

        assert_eq!(
            Ok(String::from("Database 'this_should_exist' created successfully.")),
            res
        );

        if let Ok(_) = res {
            let res = run(&["--can_connect_to_database"], &url).await;
            assert_eq!(Ok(String::from("Connection successful")), res);

            {
                let uri = postgres_url(None);
                let conn = Quaint::new(&uri).unwrap();

                conn.execute_raw("DROP DATABASE \"this_should_exist\"", &[])
                    .await
                    .unwrap();
            }

            res.unwrap();
        } else {
            res.unwrap();
        }
    }

    #[test]
    fn test_fetch_db_name() {
        let url: url::Url = "postgresql://postgres:prisma@127.0.0.1:5432/pgres?schema=test_schema"
            .parse()
            .unwrap();
        let db_name = super::fetch_db_name(&url, "postgres");
        assert_eq!(db_name, "pgres");
    }

    #[test]
    fn test_fetch_db_name_with_postgres_scheme() {
        let url: url::Url = "postgres://postgres:prisma@127.0.0.1:5432/pgres?schema=test_schema"
            .parse()
            .unwrap();
        let db_name = super::fetch_db_name(&url, "postgres");
        assert_eq!(db_name, "pgres");
    }
}
