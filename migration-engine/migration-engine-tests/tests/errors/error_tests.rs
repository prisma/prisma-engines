use indoc::formatdoc;
use migration_core::api::RpcApi;
use migration_engine_tests::sql::*;
use pretty_assertions::assert_eq;
use quaint::prelude::{Queryable, SqlFamily};
use serde_json::json;
use test_setup::*;
use url::Url;

#[tokio::test]
async fn authentication_failure_must_return_a_known_error_on_postgres() {
    let mut url: Url = postgres_10_url("test-db").parse().unwrap();

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

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let user = url.username();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(&error.render_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Authentication failed against database server at `{host}`, the provided database credentials for `postgres` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at `{host}`.", host = host),
        "meta": {
            "database_user": user,
            "database_host": host,
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn authentication_failure_must_return_a_known_error_on_mysql() {
    let mut url: Url = mysql_5_7_url("authentication_failure_must_return_a_known_error_on_mysql")
        .parse()
        .unwrap();

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

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let user = url.username();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(&error.render_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Authentication failed against database server at `{host}`, the provided database credentials for `{user}` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at `{host}`.", host = host, user = user),
        "meta": {
            "database_user": user,
            "database_host": host,
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn unreachable_database_must_return_a_proper_error_on_mysql() {
    let mut url: Url = mysql_5_7_url("unreachable_database_must_return_a_proper_error_on_mysql")
        .parse()
        .unwrap();

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

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let port = url.port().unwrap();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(&error.render_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Can't reach database server at `{host}`:`{port}`\n\nPlease make sure your database server is running at `{host}`:`{port}`.", host = host, port = port),
        "meta": {
            "database_host": host,
            "database_port": port,
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn unreachable_database_must_return_a_proper_error_on_postgres() {
    let mut url: Url = postgres_10_url("unreachable_database_must_return_a_proper_error_on_postgres")
        .parse()
        .unwrap();

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

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let host = url.host().unwrap().to_string();
    let port = url.port().unwrap();

    let json_error = serde_json::to_value(&error.render_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Can't reach database server at `{host}`:`{port}`\n\nPlease make sure your database server is running at `{host}`:`{port}`.", host = host, port = port),
        "meta": {
            "database_host": host,
            "database_port": port,
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_does_not_exist_must_return_a_proper_error() {
    let mut url: Url = mysql_5_7_url("database_does_not_exist_must_return_a_proper_error")
        .parse()
        .unwrap();
    let database_name = "notmydatabase";

    url.set_path(&format!("/{}", database_name));

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let json_error = serde_json::to_value(&error.render_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Database `{database_name}` does not exist on the database server at `{database_host}:{database_port}`.", database_name = database_name, database_host = url.host().unwrap(), database_port = url.port().unwrap()),
        "meta": {
            "database_name": database_name,
            "database_host": format!("{}", url.host().unwrap()),
            "database_port": url.port().unwrap(),
        },
        "error_code": "P1003"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_access_denied_must_return_a_proper_error_in_rpc() {
    let db_name = "dbaccessdeniedinrpc";
    let url: Url = mysql_5_7_url(db_name).parse().unwrap();
    let conn = create_mysql_database(&url).await.unwrap();

    conn.execute_raw("DROP USER IF EXISTS jeanyves", &[]).await.unwrap();
    conn.execute_raw("CREATE USER jeanyves IDENTIFIED BY '1234'", &[])
        .await
        .unwrap();

    let mut url: Url = url.clone();
    url.set_username("jeanyves").unwrap();
    url.set_password(Some("1234")).unwrap();
    url.set_path("/access_denied_test");

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url,
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();
    let json_error = serde_json::to_value(&error.render_user_facing()).unwrap();

    let expected = json!({
        "is_panic": false,
        "message": "User `jeanyves` was denied access on the database `access_denied_test`",
        "meta": {
            "database_user": "jeanyves",
            "database_name": "access_denied_test",
        },
        "error_code": "P1010",
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn bad_datasource_url_and_provider_combinations_must_return_a_proper_error() {
    let db_name = "bad_datasource_url_and_provider_combinations_must_return_a_proper_error";
    let dm = format!(
        r#"
            datasource db {{
                provider = "sqlite"
                url = "{}"
            }}
        "#,
        postgres_10_url(db_name),
    );

    let error = RpcApi::new(&dm).await.map(drop).unwrap_err();

    let json_error = serde_json::to_value(&error.render_user_facing()).unwrap();

    let err_message: String = json_error["message"].as_str().unwrap().into();

    assert!(
        err_message.contains("The URL for datasource `db` must start with the protocol `file:`"),
        "{}",
        err_message
    );

    let expected = json!({
        "is_panic": false,
        "message": err_message,
        "meta": {
            "full_error": err_message,
        },
        "error_code": "P1012",
    });

    assert_eq!(json_error, expected);
}

#[test_each_connector(tags("mysql_8"))]
async fn connections_to_system_databases_must_be_rejected(_api: &TestApi) -> TestResult {
    let names = &["", "mysql", "sys", "performance_schema"];
    for name in names {
        let dm = format!(
            r#"
                datasource db {{
                    provider = "mysql"
                    url = "{}"
                }}
            "#,
            mysql_8_url(name),
        );

        // "mysql" is the default in Quaint.
        let name = if name == &"" { "mysql" } else { name };

        let error = RpcApi::new(&dm).await.map(drop).unwrap_err();
        let json_error = serde_json::to_value(&error.render_user_facing()).unwrap();

        let expected = json!({
            "is_panic": false,
            "message": format!("The `{}` database is a system database, it should not be altered with prisma migrate. Please connect to another database.", name),
            "meta": {
                "database_name": name,
            },
            "error_code": "P3004",
        });

        assert_eq!(json_error, expected);
    }

    Ok(())
}

#[test_each_connector(tags("sqlite"))]
async fn datamodel_parser_errors_must_return_a_known_error(api: &TestApi) {
    let bad_dm = r#"
        model Test {
            id Float @id
            post Post[]
        }
    "#;

    let error = api.schema_push(bad_dm).send().await.unwrap_err().render_user_facing();

    let expected_msg = "\u{1b}[1;91merror\u{1b}[0m: \u{1b}[1mType \"Post\" is neither a built-in type, nor refers to another model, custom type, or enum.\u{1b}[0m\n  \u{1b}[1;94m-->\u{1b}[0m  \u{1b}[4mschema.prisma:4\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n\u{1b}[1;94m 3 | \u{1b}[0m            id Float @id\n\u{1b}[1;94m 4 | \u{1b}[0m            post \u{1b}[1;91mPost[]\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n";

    let expected_error = user_facing_errors::Error::from(user_facing_errors::KnownError {
        error_code: "P1012",
        message: expected_msg.into(),
        meta: serde_json::json!({ "full_error": expected_msg }),
    });

    assert_eq!(error, expected_error);
}

#[test_each_connector]
async fn unique_constraint_errors_in_migrations_must_return_a_known_error(api: &TestApi) -> TestResult {
    use quaint::ast::*;

    let dm = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    let insert = Insert::multi_into(api.render_table_name("Fruit"), &["name"])
        .values(("banana",))
        .values(("apple",))
        .values(("banana",));

    api.database().execute(insert.into()).await?;

    let dm2 = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String @unique
        }
    "#;

    let res = api
        .schema_push(dm2)
        .force(true)
        .migration_id(Some("the-migration"))
        .send()
        .await
        .unwrap_err()
        .render_user_facing();

    let json_error = serde_json::to_value(&res).unwrap();

    let expected_msg = match api.sql_family() {
        SqlFamily::Mysql => "Unique constraint failed on the constraint: `name_unique`",
        SqlFamily::Mssql => "Unique constraint failed on the constraint: `Fruit_name_unique`",
        _ => "Unique constraint failed on the fields: (`name`)",
    };

    let expected_target = match api.sql_family() {
        SqlFamily::Mysql => json!("name_unique"),
        SqlFamily::Mssql => json!("Fruit_name_unique"),
        _ => json!(["name"]),
    };

    let expected_json = json!({
        "is_panic": false,
        "message": expected_msg,
        "meta": {
            "target": expected_target,
        },
        "error_code": "P2002",
    });

    assert_eq!(json_error, expected_json);

    Ok(())
}

#[test_each_connector(tags("mysql_5_6"))]
async fn json_fields_must_be_rejected(api: &TestApi) -> TestResult {
    let dm = format!(
        r#"
        {}

        model Test {{
            id Int @id
            j Json
        }}

        "#,
        api.datasource()
    );

    let result = api
        .schema_push(dm)
        .send()
        .await
        .unwrap_err()
        .render_user_facing()
        .unwrap_known();

    assert_eq!(result.error_code, "P1015");
    assert!(result
        .message
        .contains("Your Prisma schema is using features that are not supported for the version of the database"));
    assert!(result
        .message
        .contains("- The `Json` data type used in Test.j is not supported on MySQL 5.6.\n"));

    Ok(())
}

#[tokio::test]
async fn connection_string_problems_give_a_nice_error() {
    let providers = &[
        ("mysql", "mysql://root:password-with-#@localhost:3306/database"),
        (
            "postgresql",
            "postgresql://root:password-with-#@localhost:5432/postgres",
        ),
        ("sqlserver", "sqlserver://root:password-with-#@localhost:5432/postgres"),
    ];

    for provider in providers {
        let dm = formatdoc!(
            r#"
                datasource db {{
                  provider = "{}"
                  url = "{}"
                }}
        "#,
            provider.0,
            provider.1
        );

        let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

        let json_error = serde_json::to_value(&error.render_user_facing()).unwrap();

        let details = match provider.0 {
            "sqlserver" => {
                formatdoc!(
                    "Error parsing connection string: Conversion error: invalid digit found in string in `{connection_string}`.
                    Please refer to the documentation in https://www.prisma.io/docs/reference/database-reference/connection-urls
                    for constructing a correct connection string. In some cases, certain characters must be escaped.
                    Please check the string for any illegal characters.",
                    connection_string = provider.1
                ).replace('\n', " ")
            },
            _ => {
                formatdoc!(
                    "Error parsing connection string: invalid port number in `{connection_string}`.
                    Please refer to the documentation in https://www.prisma.io/docs/reference/database-reference/connection-urls
                    for constructing a correct connection string. In some cases, certain characters must be escaped.
                    Please check the string for any illegal characters.",
                    connection_string = provider.1
                ).replace('\n', " ")
            }
        };

        let expected = json!({
            "is_panic": false,
            "message": format!("The provided database string is invalid. {}", &details),
            "meta": {
                "details": &details,
            },
            "error_code": "P1013"
        });

        assert_eq!(expected, json_error);
    }
}
