use quaint::{prelude::*, single::Quaint};
use serde_json::json;
use test_setup::*;
use url::Url;

#[tokio::test]
async fn database_already_exists_must_return_a_proper_error() {
    let db_name = "database_already_exists_must_return_a_proper_error";
    let url = postgres_10_url(db_name);

    let conn = Quaint::new(&postgres_10_url("postgres")).await.unwrap();
    conn.execute_raw(
        "CREATE DATABASE \"database_already_exists_must_return_a_proper_error\"",
        &[],
    )
    .await
    .ok();

    let error = get_cli_error(&["migration-engine", "cli", "--datasource", &url, "create-database"]).await;

    let (host, port) = {
        let url = Url::parse(&url).unwrap();
        (url.host().unwrap().to_string(), url.port().unwrap())
    };

    let json_error = serde_json::to_value(&error).unwrap();

    let expected = json!({
        "is_panic": false,
        "message": format!("Database `database_already_exists_must_return_a_proper_error` already exists on the database server at `{host}:{port}`", host = host, port = port),
        "meta": {
            "database_name": "database_already_exists_must_return_a_proper_error",
            "database_host": host,
            "database_port": port,
        },
        "error_code": "P1009"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn bad_postgres_url_must_return_a_good_error() {
    let url = "postgresql://postgres:prisma@localhost:543`/mydb?schema=public";

    let error = get_cli_error(&["migration-engine", "cli", "--datasource", &url, "create-database"]).await;

    let json_error = serde_json::to_value(&error).unwrap();

    let expected = json!({
        "is_panic": false,
        "message": "Error parsing connection string: invalid port number in `postgresql://postgres:prisma@localhost:543`/mydb?schema=public`)\n",
        "backtrace": null,
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_access_denied_must_return_a_proper_error_in_cli() {
    let db_name = "dbaccessdeniedincli";
    let url: Url = mysql_url(db_name).parse().unwrap();
    let conn = create_mysql_database(&url).await.unwrap();

    conn.execute_raw("DROP USER IF EXISTS jeanmichel", &[]).await.unwrap();
    conn.execute_raw("CREATE USER jeanmichel IDENTIFIED BY '1234'", &[])
        .await
        .unwrap();

    let mut url: Url = url.clone();
    url.set_username("jeanmichel").unwrap();
    url.set_password(Some("1234")).unwrap();
    url.set_path("/access_denied_test");

    let error = get_cli_error(&[
        "migration-engine",
        "cli",
        "--datasource",
        url.as_str(),
        "can-connect-to-database",
    ])
    .await;

    let json_error = serde_json::to_value(&error).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": "User `jeanmichel` was denied access on the database `access_denied_test`",
        "meta": {
            "database_user": "jeanmichel",
            "database_name": "access_denied_test",
        },
        "error_code": "P1010",
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn tls_errors_must_be_mapped_in_the_cli() {
    let url = format!(
        "{}&sslmode=require&sslaccept=strict",
        postgres_10_url("tls_errors_must_be_mapped_in_the_cli")
    );
    let error = get_cli_error(&[
        "migration-engine",
        "cli",
        "--datasource",
        &url,
        "can-connect-to-database",
    ])
    .await;

    let json_error = serde_json::to_value(&error).unwrap();

    let expected = json!({
        "is_panic": false,
        "message": format!("Error opening a TLS connection: error performing TLS handshake: server does not support TLS"),
        "meta": {
            "message": "error performing TLS handshake: server does not support TLS",
        },
        "error_code": "P1011"
    });

    assert_eq!(json_error, expected);
}

async fn get_cli_error(cli_args: &[&str]) -> user_facing_errors::Error {
    use structopt::StructOpt;

    let matches = crate::MigrationEngineCli::from_iter(cli_args.iter());
    let cli_command = matches.cli_subcommand.expect("cli subcommand is passed");
    cli_command
        .unwrap_cli()
        .run_inner()
        .await
        .map_err(|err| crate::commands::error::render_error(err))
        .unwrap_err()
}
