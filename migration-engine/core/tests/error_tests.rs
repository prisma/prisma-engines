mod test_harness;

use migration_connector::steps::{DeleteModel, MigrationStep};
use migration_core::{
    api::{render_error, RpcApi},
    cli,
    commands::{ApplyMigrationCommand, ApplyMigrationInput},
};
use pretty_assertions::assert_eq;
use serde_json::json;
use sql_connection::SyncSqlConnection;
use test_harness::*;
use url::Url;

#[test]
fn authentication_failure_must_return_a_known_error_on_postgres() {
    let mut url: Url = postgres_url().parse().unwrap();

    url.set_password(Some("obviously-not-right")).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "postgres"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new_async(&dm).map(|_| ()).unwrap_err();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "message": "Authentication failed against database server at `127.0.0.1`, the provided database credentials for `postgres` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at `127.0.0.1`.",
        "meta": {
            "database_user": "postgres",
            "database_host": "127.0.0.1",
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

#[test]
fn authentication_failure_must_return_a_known_error_on_mysql() {
    let mut url: Url = mysql_url().parse().unwrap();

    url.set_password(Some("obviously-not-right")).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new_async(&dm).map(|_| ()).unwrap_err();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "message": "Authentication failed against database server at `127.0.0.1`, the provided database credentials for `root` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at `127.0.0.1`.",
        "meta": {
            "database_user": "root",
            "database_host": "127.0.0.1",
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

#[test]
fn unreachable_database_must_return_a_proper_error_on_mysql() {
    let mut url: Url = mysql_url().parse().unwrap();

    url.set_port(Some(8787)).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new_async(&dm).map(|_| ()).unwrap_err();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "message": "Can't reach database server at `127.0.0.1`:`8787`\n\nPlease make sure your database server is running at `127.0.0.1`:`8787`.",
        "meta": {
            "database_port": "8787",
            "database_host": "127.0.0.1",
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}

#[test]
fn unreachable_database_must_return_a_proper_error_on_postgres() {
    let mut url: Url = postgres_url().parse().unwrap();

    url.set_port(Some(8787)).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "postgres"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new_async(&dm).map(|_| ()).unwrap_err();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "message": "Can't reach database server at `127.0.0.1`:`8787`\n\nPlease make sure your database server is running at `127.0.0.1`:`8787`.",
        "meta": {
            "database_port": "8787",
            "database_host": "127.0.0.1",
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}

#[test]
fn database_does_not_exist_must_return_a_proper_error() {
    let mut url: Url = mysql_url().parse().unwrap();

    url.set_path("notmydatabase");

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new_async(&dm).map(|_| ()).unwrap_err();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "message": "Database `notmydatabase` does not exist on the database server at `127.0.0.1:3306`.",
        "meta": {
            "database_name": "notmydatabase",
            "database_schema_name": null,
            "database_location": "127.0.0.1:3306",
        },
        "error_code": "P1003"
    });

    assert_eq!(json_error, expected);
}

#[test]
fn database_already_exists_must_return_a_proper_error() {
    let error = get_cli_error(&[
        "migration-engine",
        "cli",
        "--datasource",
        &postgres_url(),
        "--create_database",
    ]);
    let json_error = serde_json::to_value(&error).unwrap();
    let expected = json!({
        "message": "Database `test-db` already exists on the database server at `127.0.0.1:5432`",
        "meta": {
            "database_name": "test-db",
            "database_host": "127.0.0.1",
            "database_port": 5432,
        },
        "error_code": "P1009"
    });

    assert_eq!(json_error, expected);
}

#[test]
fn database_access_denied_must_return_a_proper_error_in_cli() {
    let conn = sql_connection::GenericSqlConnection::from_database_str(&mysql_url(), None).unwrap();

    conn.execute_raw("DROP USER IF EXISTS jeanmichel", &[]).unwrap();
    conn.execute_raw("CREATE USER jeanmichel IDENTIFIED BY '1234'", &[])
        .unwrap();

    let mut url: Url = mysql_url().parse().unwrap();
    url.set_username("jeanmichel").unwrap();
    url.set_password(Some("1234")).unwrap();
    url.set_path("access_denied_test");

    let error = get_cli_error(&[
        "migration-engine",
        "cli",
        "--datasource",
        url.as_str(),
        "--can_connect_to_database",
    ]);

    let json_error = serde_json::to_value(&error).unwrap();
    let expected = json!({
        "message": "User `jeanmichel` was denied access on the database `access_denied_test`",
        "meta": {
            "database_user": "jeanmichel",
            "database_name": "access_denied_test",
        },
        "error_code": "P1010",
    });

    assert_eq!(json_error, expected);
}

#[test]
fn database_access_denied_must_return_a_proper_error_in_rpc() {
    let conn = sql_connection::GenericSqlConnection::from_database_str(&mysql_url(), None).unwrap();

    conn.execute_raw("DROP USER IF EXISTS jeanmichel", &[]).unwrap();
    conn.execute_raw("CREATE USER jeanmichel IDENTIFIED BY '1234'", &[])
        .unwrap();

    let mut url: Url = mysql_url().parse().unwrap();
    url.set_username("jeanmichel").unwrap();
    url.set_password(Some("1234")).unwrap();
    url.set_path("access_denied_test");

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url,
    );

    let error = RpcApi::new_async(&dm).map(|_| ()).unwrap_err();
    let json_error = serde_json::to_value(&render_error(error)).unwrap();

    let expected = json!({
        "message": "User `jeanmichel` was denied access on the database `access_denied_test`",
        "meta": {
            "database_user": "jeanmichel",
            "database_name": "access_denied_test",
        },
        "error_code": "P1010",
    });

    assert_eq!(json_error, expected);
}

#[test_one_connector(connector = "postgres")]
fn command_errors_must_return_an_unknown_error(api: &TestApi) {
    let input = ApplyMigrationInput {
        migration_id: "the-migration".to_owned(),
        steps: vec![MigrationStep::DeleteModel(DeleteModel {
            model: "abcd".to_owned(),
        })],
        force: Some(true),
    };

    let error = api.execute_command::<ApplyMigrationCommand>(&input).unwrap_err();

    let expected_error = user_facing_errors::Error::Unknown(user_facing_errors::UnknownError {
        message: "Failure during a migration command: Generic error. (code: 1, error: The model abcd does not exist in this Datamodel. It is not possible to delete it.)".to_owned(),
        backtrace: None,
    });

    assert_eq!(error, expected_error);
}

fn get_cli_error(cli_args: &[&str]) -> user_facing_errors::Error {
    let app = cli::clap_app();
    let matches = app.get_matches_from(cli_args);
    let cli_matches = matches.subcommand_matches("cli").expect("cli subcommand is passed");
    let database_url = cli_matches.value_of("datasource").expect("datasource is provided");
    cli::run(&cli_matches, database_url)
        .map_err(|err| cli::render_error(err))
        .unwrap_err()
}
