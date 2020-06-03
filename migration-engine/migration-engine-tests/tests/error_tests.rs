use migration_connector::steps::{DeleteModel, MigrationStep};
use migration_core::api::{render_error, RpcApi};
use migration_engine_tests::sql::*;
use pretty_assertions::assert_eq;
use quaint::prelude::*;
use serde_json::json;
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

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
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
    let mut url: Url = mysql_url("authentication_failure_must_return_a_known_error_on_mysql")
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

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
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
    let mut url: Url = mysql_url("unreachable_database_must_return_a_proper_error_on_mysql")
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

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
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

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
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
    let mut url: Url = mysql_url("database_does_not_exist_must_return_a_proper_error")
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

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
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
    let url: Url = mysql_url(db_name).parse().unwrap();
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
    let json_error = serde_json::to_value(&render_error(error)).unwrap();

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

    let json_error = serde_json::to_value(&render_error(error)).unwrap();

    let expected = json!({
        "is_panic": false,
        "message": "Error in datamodel: Error validating datasource `db`: The URL for datasource `db` must start with the protocol `sqlite://`.",
        "backtrace": null,
    });

    assert_eq!(json_error, expected);
}

#[test_each_connector(tags("mysql_8"))]
async fn connections_to_system_databases_must_be_rejected(_api: &TestApi) -> TestResult {
    let names = &["", "mysql", "sys", "performance_schema"];
    for name in names {
        dbg!(name);
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

        let json_error = serde_json::to_value(&render_error(error)).unwrap();

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
async fn command_errors_must_return_an_unknown_error(api: &TestApi) {
    let steps = vec![MigrationStep::DeleteModel(DeleteModel {
        model: "abcd".to_owned(),
    })];

    let error = api
        .apply()
        .migration_id(Some("the-migration"))
        .steps(Some(steps))
        .force(Some(true))
        .send_user_facing()
        .await
        .unwrap_err();

    let expected_error = user_facing_errors::Error::from(user_facing_errors::UnknownError {
        message: "Failure during a migration command: Generic error. (error: The model abcd does not exist in this Datamodel. It is not possible to delete it.)".to_owned(),
        backtrace: None,
    });

    assert_eq!(error, expected_error);
}

#[test_each_connector(tags("sqlite"))]
async fn datamodel_parser_errors_must_return_a_known_error(api: &TestApi) {
    let bad_dm = r#"
        model Test {
            id Float @id
            post Post[]
        }
    "#;

    let error = api.infer_apply(bad_dm).send_user_facing().await.unwrap_err();

    let expected_msg = "\u{1b}[1;91merror\u{1b}[0m: \u{1b}[1mType \"Post\" is neither a built-in type, nor refers to another model, custom type, or enum.\u{1b}[0m\n  \u{1b}[1;94m-->\u{1b}[0m  \u{1b}[4mschema.prisma:4\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n\u{1b}[1;94m 3 | \u{1b}[0m            id Float @id\n\u{1b}[1;94m 4 | \u{1b}[0m            post \u{1b}[1;91mPost[]\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n";

    let expected_error = user_facing_errors::Error::from(user_facing_errors::KnownError {
        error_code: "P1012".into(),
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

    api.infer_and_apply(dm).await;

    let insert = Insert::multi_into(api.render_table_name("Fruit"), &["name"])
        .values(("banana",))
        .values(("apple",))
        .values(("banana",));

    api.database().execute(insert.into()).await.unwrap();

    let dm2 = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String @unique
        }
    "#;

    let steps = api
        .infer(dm2)
        .migration_id(Some("the-migration"))
        .send()
        .await?
        .datamodel_steps;

    let error = api
        .apply()
        .steps(Some(steps))
        .force(Some(true))
        .migration_id(Some("the-migration"))
        .send_user_facing()
        .await
        .unwrap_err();

    let json_error = serde_json::to_value(&error).unwrap();

    let expected_msg = if api.sql_family().is_mysql() {
        "Unique constraint failed on the constraint: `name`"
    } else {
        "Unique constraint failed on the fields: (`name`)"
    };
    let expected_target = if api.sql_family().is_mysql() {
        json!("name")
    } else {
        json!(["name"])
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

    let result = api.infer(dm).send().await?;

    assert_eq!(
        result
            .errors
            .into_iter()
            .map(|error| error.description.clone())
            .collect::<Vec<String>>(),
        &["The `Json` data type used in Test.j is not supported on MySQL 5.6."]
    );

    Ok(())
}
