use connection_string::JdbcString;
use std::process::{Command, Output};
use test_macros::test_connector;
use test_setup::{BitFlags, Tags, TestApiArgs};
use url::Url;
use user_facing_errors::{common::DatabaseDoesNotExist, UserFacingError};

struct TestApi {
    args: TestApiArgs,
}

impl TestApi {
    fn new(args: TestApiArgs) -> Self {
        TestApi { args }
    }

    fn connection_string(&self) -> String {
        let rt = test_setup::runtime::test_tokio_runtime();
        let args = &self.args;

        if args.tags().contains(Tags::Postgres) {
            rt.block_on(args.create_postgres_database()).2
        } else if args.tags().contains(Tags::Mysql) {
            rt.block_on(args.create_mysql_database()).1
        } else if args.tags().contains(Tags::Mssql) {
            rt.block_on(args.create_mssql_database()).1
        } else {
            unreachable!()
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_migration-engine"))
            .arg("cli")
            .args(args)
            .env("RUST_LOG", "INFO")
            .output()
            .unwrap()
    }
}

#[test_connector(tags(Mysql))]
fn test_connecting_with_a_working_mysql_connection_string(api: TestApi) {
    let connection_string = api.connection_string();
    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);

    assert!(output.status.success(), "{:?}", output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{:?}", stderr);
}

#[test_connector(tags(Mysql))]
fn test_connecting_with_a_non_working_mysql_connection_string(api: TestApi) {
    let mut non_existing_url: url::Url = api.args.database_url().parse().unwrap();

    non_existing_url.set_path("this_does_not_exist");

    let output = api.run(&["--datasource", &non_existing_url.to_string(), "can-connect-to-database"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1003""#), "{}", stderr);
}

#[test_connector(tags(Postgres))]
fn test_connecting_with_a_working_postgres_connection_string(api: TestApi) {
    let conn_string = if api.args.database_url().starts_with("postgres:") {
        api.args.database_url().replacen("postgres:", "postgresql:", 1)
    } else {
        api.args.database_url().to_owned()
    };

    let output = api.run(&["--datasource", &conn_string, "can-connect-to-database"]);

    assert!(output.status.success(), "{:?}", output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{:?}", stderr);
}

// Note: not redundant with previous test because of the different URL scheme.
#[test_connector(tags(Postgres))]
fn test_connecting_with_a_working_postgresql_connection_string(api: TestApi) {
    let conn_string = if api.args.database_url().starts_with("postgresql:") {
        api.args.database_url().replacen("postgresql:", "postgres:", 1)
    } else {
        api.args.database_url().to_owned()
    };

    let output = api.run(&["--datasource", &conn_string, "can-connect-to-database"]);

    assert!(output.status.success(), "{:?}", output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{:?}", stderr);
}

#[test_connector(tags(Postgres))]
fn test_connecting_with_a_non_working_psql_connection_string(api: TestApi) {
    let mut url: url::Url = api.args.database_url().parse().unwrap();
    url.set_path("this_does_not_exist");

    let output = api.run(&["--datasource", &url.to_string(), "can-connect-to-database"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1003""#), "{}", stderr);
}

#[test_connector(tags(Mssql))]
fn test_connecting_with_a_working_mssql_connection_string(api: TestApi) {
    let connection_string = api.connection_string();

    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);

    assert!(output.status.success(), "{:?}", output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Connection successful"), "{:?}", stderr);
}

#[test_connector(tags(Postgres, Mysql))]
fn test_create_database(api: TestApi) {
    let connection_string = api.connection_string();
    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success());

    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Database 'test_create_database\' was successfully created."));

    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);
    assert!(output.status.success());
}

#[test_connector(tags(Mssql))]
fn test_create_database_mssql(api: TestApi) {
    let connection_string = api
        .connection_string()
        .replace("master", "masterNEW")
        .replace("test_create_database_mssql", "test_create_database_NEW");

    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success());

    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Database 'masterNEW\' was successfully created."));

    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);
    assert!(output.status.success());
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
    let output = api.run(&["--datasource", &url, "create-database"]);
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("success"));
    assert!(stderr.contains("test_create_sqlite_database.db"));

    assert!(sqlite_path.exists());
}

#[test_connector(tags(Sqlite))]
fn test_drop_sqlite_database(api: TestApi) {
    let base_dir = tempfile::tempdir().unwrap();
    let sqlite_path = base_dir.path().join("test.db");
    let url = format!("file:{}", sqlite_path.to_string_lossy());

    let output = api.run(&["--datasource", &url, "create-database"]);
    assert!(output.status.success());
    let output = api.run(&["--datasource", &url, "can-connect-to-database"]);
    assert!(output.status.success());
    let output = api.run(&["--datasource", &url, "drop-database"]);
    assert!(output.status.success());
    assert!(!sqlite_path.exists());
}

#[test_connector(tags(Postgres, Mysql))]
fn test_drop_database(api: TestApi) {
    let connection_string = api.connection_string();
    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    println!("{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success());

    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);
    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(DatabaseDoesNotExist::ERROR_CODE));
}

#[test_connector(tags(Mssql))]
fn test_drop_sqlserver_database(api: TestApi) {
    let mut connection_string: JdbcString = format!("jdbc:{}", api.connection_string()).parse().unwrap();

    connection_string
        .properties_mut()
        .insert(String::from("database"), String::from("NEWDATABASE"));

    let connection_string = connection_string.to_string().replace("jdbc:", "");

    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert!(output.status.success());

    let output = api.run(&["--datasource", &connection_string, "drop-database"]);
    assert!(output.status.success());

    let output = api.run(&["--datasource", &connection_string, "can-connect-to-database"]);
    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(DatabaseDoesNotExist::ERROR_CODE));
}

#[test_connector(tags(Postgres))]
fn bad_postgres_url_must_return_a_good_error(api: TestApi) {
    let url = "postgresql://postgres:prisma@localhost:543`/mydb?schema=public";

    let output = api.run(&["--datasource", url, "create-database"]);
    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1013""#));
    assert!(stderr.contains("invalid port number",));
}

#[test_connector(tags(Postgres))]
fn database_already_exists_must_return_a_proper_error(api: TestApi) {
    let connection_string = api.connection_string();
    let output = api.run(&["--datasource", &connection_string, "create-database"]);
    assert_eq!(output.status.code(), Some(1));

    let (host, port) = {
        let url = Url::parse(&connection_string).unwrap();
        (url.host().unwrap().to_string(), url.port().unwrap())
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1009""#));
    assert!(stderr.contains(&format!("Database `database_already_exists_must_return_a_proper_error` already exists on the database server at `{host}:{port}`", host = host, port = port)));
}

#[test_connector(tags(Postgres))]
fn tls_errors_must_be_mapped_in_the_cli(api: TestApi) {
    let connection_string = api.connection_string();
    let url = format!("{}&sslmode=require&sslaccept=strict", connection_string);
    let output = api.run(&["--datasource", &url, "can-connect-to-database"]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(r#""error_code":"P1011""#));
    assert!(
        stderr.contains("Error opening a TLS connection: error performing TLS handshake: server does not support TLS")
    );
}
