use migration_connector::ConnectorError;
use structopt::StructOpt;
use test_macros::test_connector;
use test_setup::{sqlite_test_url, BitFlags, Tags, TestApiArgs};
use url::Url;
use user_facing_errors::{common::DatabaseDoesNotExist, UserFacingError};

struct TestApi {
    connection_string: String,
    rt: tokio::runtime::Runtime,
}

impl TestApi {
    fn new(args: TestApiArgs) -> Self {
        let rt = test_setup::runtime::test_tokio_runtime();

        let connection_string = if args.tags().contains(Tags::Postgres) {
            rt.block_on(args.create_postgres_database()).2
        } else if args.tags().contains(Tags::Mysql) {
            rt.block_on(args.create_mysql_database()).1
        } else if args.tags().contains(Tags::Sqlite) {
            sqlite_test_url(args.test_function_name())
        } else {
            unreachable!()
        };

        TestApi { connection_string, rt }
    }

    fn run(&self, args: &[&str]) -> Result<String, ConnectorError> {
        let cli = super::Cli::from_iter(std::iter::once(&"migration-engine-cli-test").chain(args.iter()));
        self.rt.block_on(cli.run_inner())
    }

    fn get_cli_error(&self, cli_args: &[&str]) -> ConnectorError {
        let matches = crate::MigrationEngineCli::from_iter(cli_args.iter());
        let cli_command = matches.cli_subcommand.expect("cli subcommand is passed");
        self.rt.block_on(cli_command.unwrap_cli().run_inner()).unwrap_err()
    }
}

#[test_connector(tags(Mysql))]
fn test_connecting_with_a_working_mysql_connection_string(api: TestApi) {
    let result = api
        .run(&["--datasource", &api.connection_string, "can-connect-to-database"])
        .unwrap();

    assert_eq!(result, "Connection successful");
}

#[test_connector(tags(Mysql))]
fn test_connecting_with_a_non_working_mysql_connection_string(api: TestApi) {
    let mut non_existing_url: url::Url = api.connection_string.parse().unwrap();

    non_existing_url.set_path("this_does_not_exist");

    let err = api
        .run(&["--datasource", &non_existing_url.to_string(), "can-connect-to-database"])
        .unwrap_err();

    assert_eq!("P1003", err.error_code().unwrap());
}

#[test_connector(tags(Postgres))]
fn test_connecting_with_a_working_postgres_connection_string(api: TestApi) {
    let conn_string = if api.connection_string.starts_with("postgres:") {
        api.connection_string.replacen("postgres:", "postgresql:", 1)
    } else {
        api.connection_string.clone()
    };

    let result = api
        .run(&["--datasource", &conn_string, "can-connect-to-database"])
        .unwrap();

    assert_eq!(result, "Connection successful");
}

// Note: not redundant with previous test because of the different URL scheme.
#[test_connector(tags(Postgres))]
fn test_connecting_with_a_working_postgresql_connection_string(api: TestApi) {
    let conn_string = if api.connection_string.starts_with("postgresql:") {
        api.connection_string.replacen("postgresql:", "postgres:", 1)
    } else {
        api.connection_string.clone()
    };

    let result = api
        .run(&["--datasource", &conn_string, "can-connect-to-database"])
        .unwrap();

    assert_eq!(result, "Connection successful");
}

#[test_connector(tags(Postgres))]
fn test_connecting_with_a_non_working_psql_connection_string(api: TestApi) {
    let mut url: url::Url = api.connection_string.parse().unwrap();
    url.set_path("this_does_not_exist");

    let err = api
        .run(&["--datasource", &url.to_string(), "can-connect-to-database"])
        .unwrap_err();

    assert_eq!("P1003", err.error_code().unwrap());
}

#[test_connector(tags(Postgres, Mysql))]
fn test_create_database(api: TestApi) {
    api.run(&["--datasource", &api.connection_string, "drop-database"])
        .unwrap();

    let res = api
        .run(&["--datasource", &api.connection_string, "create-database"])
        .unwrap();

    assert_eq!("Database 'test_create_database\' was successfully created.", res);

    let res = api.run(&["--datasource", &api.connection_string, "can-connect-to-database"]);
    assert_eq!("Connection successful", res.as_ref().unwrap());
}

#[test_connector(tags(Sqlite))]
fn test_create_sqlite_database(api: TestApi) {
    let base_dir = tempfile::tempdir().unwrap();

    let sqlite_path = base_dir
        .path()
        .join("doesntexist/either")
        .join("test_create_sqlite_database.db");

    assert!(!sqlite_path.exists());

    let url = format!("file:{}", sqlite_path.to_string_lossy());
    let res = api.run(&["--datasource", &url, "create-database"]);
    let msg = res.as_ref().unwrap();

    assert!(msg.contains("success"));
    assert!(msg.contains("test_create_sqlite_database.db"));

    assert!(sqlite_path.exists());
}

#[test_connector(tags(Sqlite))]
fn test_drop_sqlite_database(api: TestApi) {
    let base_dir = tempfile::tempdir().unwrap();
    let sqlite_path = base_dir.path().join("test.db");
    let url = format!("file:{}", sqlite_path.to_string_lossy());

    api.run(&["--datasource", &url, "create-database"]).unwrap();
    api.run(&["--datasource", &url, "can-connect-to-database"]).unwrap();
    api.run(&["--datasource", &url, "drop-database"]).unwrap();
    assert!(!sqlite_path.exists());
}

#[test_connector(tags(Mysql, Postgres))]
fn test_drop_database(api: TestApi) {
    api.run(&["--datasource", &api.connection_string, "drop-database"])
        .unwrap();

    let err = api
        .run(&["--datasource", &api.connection_string, "can-connect-to-database"])
        .unwrap_err();

    assert_eq!(err.error_code(), Some(DatabaseDoesNotExist::ERROR_CODE));
}

#[test_connector(tags(Postgres))]
fn database_already_exists_must_return_a_proper_error(api: TestApi) {
    let error = api.get_cli_error(&[
        "migration-engine",
        "cli",
        "--datasource",
        &api.connection_string,
        "create-database",
    ]);

    let (host, port) = {
        let url = Url::parse(&api.connection_string).unwrap();
        (url.host().unwrap().to_string(), url.port().unwrap())
    };

    assert_eq!(error.error_code(), Some("P1009"));
    assert_eq!(error.to_string(), format!("Database `database_already_exists_must_return_a_proper_error` already exists on the database server at `{host}:{port}`\n", host = host, port = port));
}

#[test_connector(tags(Postgres))]
fn bad_postgres_url_must_return_a_good_error(api: TestApi) {
    let url = "postgresql://postgres:prisma@localhost:543`/mydb?schema=public";

    let error = api.get_cli_error(&["migration-engine", "cli", "--datasource", url, "create-database"]);

    assert_eq!(
        error.to_string(),
        "Error parsing connection string: invalid port number in database URL\n"
    );
}

#[test_connector(tags(Postgres))]
fn tls_errors_must_be_mapped_in_the_cli(api: TestApi) {
    let url = format!("{}&sslmode=require&sslaccept=strict", api.connection_string);
    let error = api.get_cli_error(&[
        "migration-engine",
        "cli",
        "--datasource",
        &url,
        "can-connect-to-database",
    ]);

    assert_eq!(error.error_code(), Some("P1011"));
    assert_eq!(
        error.to_string(),
        "Error opening a TLS connection: error performing TLS handshake: server does not support TLS\n"
    );
}
