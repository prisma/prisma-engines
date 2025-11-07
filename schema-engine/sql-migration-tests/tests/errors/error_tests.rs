use connection_string::JdbcString;
use indoc::{formatdoc, indoc};
use pretty_assertions::assert_eq;
use quaint::prelude::Insert;
use schema_core::{
    DatasourceUrls,
    json_rpc::types::{DatasourceParam, EnsureConnectionValidityParams, SchemasContainer},
    schema_connector::ConnectorError,
};
use serde_json::json;
use sql_migration_tests::test_api::*;
use std::str::FromStr;
use url::Url;

pub(crate) async fn connection_error(url: impl Into<String>, schema: String) -> ConnectorError {
    let mut api =
        match schema_core::schema_api_without_extensions(Some(schema.clone()), DatasourceUrls::from_url(url), None) {
            Ok(api) => api,
            Err(err) => return err,
        };

    let err = api
        .ensure_connection_validity(EnsureConnectionValidityParams {
            datasource: DatasourceParam::Schema(SchemasContainer {
                files: vec![SchemaContainer {
                    path: "schema.prisma".to_string(),
                    content: schema,
                }],
            }),
        })
        .await
        .unwrap_err();

    // The type of the error here fits the return type of the function, but it's a different error semantically!
    // Since it's not expected to fail, we can just unwrap here.
    api.dispose().await.unwrap();

    err
}

#[test_connector(tags(Postgres12))]
fn authentication_failure_must_return_a_known_error_on_postgres(api: TestApi) {
    let mut db_url: Url = api.connection_string().parse().unwrap();

    db_url.set_password(Some("obviously-not-right")).unwrap();

    let dm = r#"
        datasource db {
            provider = "postgres"
        }
    "#
    .into();

    let error = tok(connection_error(db_url.as_str(), dm));

    let user = db_url.username();

    let json_error = serde_json::to_value(error.to_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Authentication failed against database server, the provided database credentials for `postgres` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at the configured address."),
        "meta": {
            "database_user": user,
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

#[test_connector(tags(Mysql), exclude(Vitess))]
fn authentication_failure_must_return_a_known_error_on_mysql(api: TestApi) {
    let mut url: Url = api.connection_string().parse().unwrap();

    url.set_password(Some("obviously-not-right")).unwrap();

    let dm = r#"
        datasource db {
            provider = "mysql"
        }
    "#
    .into();

    let error = tok(connection_error(url.as_str(), dm));

    let user = url.username();

    let json_error = serde_json::to_value(error.to_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Authentication failed against database server, the provided database credentials for `{user}` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at the configured address."),
        "meta": {
            "database_user": user,
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

#[test_connector(tags(Mssql))]
fn authentication_failure_must_return_a_known_error_on_mssql(api: TestApi) {
    let mut url = JdbcString::from_str(&format!("jdbc:{}", api.connection_string())).unwrap();
    let properties = url.properties_mut();
    let user = properties.get("user").cloned().unwrap();

    *properties.get_mut("password").unwrap() = "obviously-not-right".to_string();

    let dm = r#"
        datasource db {
            provider = "sqlserver"
        }
    "#
    .into();

    let error = tok(connection_error(url.to_string().replace("jdbc:", ""), dm));

    let json_error = serde_json::to_value(error.to_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Authentication failed against database server, the provided database credentials for `{user}` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at the configured address."),
        "meta": {
            "database_user": user,
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

// TODO(tech-debt): get rid of provider-specific PSL `dm` declaration, and use `test_api::datamodel_with_provider` utility instead.
// See: https://github.com/prisma/team-orm/issues/835.
// This issue also currently prevents us from defining an `Mssql`-specific copy of this `unreachable_database_*` test case,
// due to url parsing differences between the `url` crate and `quaint`'s `MssqlUrl` struct.
#[test_connector(tags(Mysql))]
fn unreachable_database_must_return_a_proper_error_on_mysql(api: TestApi) {
    let mut url: Url = api.connection_string().parse().unwrap();

    url.set_port(Some(8787)).unwrap();

    let dm = r#"
        datasource db {
            provider = "mysql"
        }
    "#
    .into();

    let error = tok(connection_error(url.as_str(), dm));

    let port = url.port().unwrap();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(error.to_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Can't reach database server at `{host}:{port}`\n\nPlease make sure your database server is running at `{host}:{port}`."),
        "meta": {
            "database_location": format!("{host}:{port}"),
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}

#[test_connector(tags(Postgres12))]
fn unreachable_database_must_return_a_proper_error_on_postgres(api: TestApi) {
    let mut url: Url = api.connection_string().parse().unwrap();

    url.set_port(Some(8787)).unwrap();

    let dm = r#"
        datasource db {
            provider = "postgres"
        }
    "#
    .into();

    let error = tok(connection_error(url.as_str(), dm));

    let host = url.host().unwrap().to_string();
    let port = url.port().unwrap();

    let json_error = serde_json::to_value(error.to_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Can't reach database server at `{host}:{port}`\n\nPlease make sure your database server is running at `{host}:{port}`."),
        "meta": {
            "database_location": format!("{host}:{port}"),
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}

#[test_connector(tags(Mysql), exclude(Vitess))]
fn database_does_not_exist_must_return_a_proper_error(api: TestApi) {
    let mut url: Url = api.connection_string().parse().unwrap();
    let database_name = "notmydatabase";

    url.set_path(&format!("/{database_name}"));

    let dm = r#"
        datasource db {
            provider = "mysql"
        }
    "#
    .into();

    let error = tok(connection_error(url.as_str(), dm));

    let json_error = serde_json::to_value(error.to_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Database `{database_name}` does not exist", database_name = database_name),
        "meta": {
            "database_name": database_name,
        },
        "error_code": "P1003"
    });

    assert_eq!(json_error, expected);
}

#[test_connector(tags(Vitess))]
fn database_does_not_exist_must_return_a_proper_error_in_vitess(api: TestApi) {
    let mut url: Url = api.connection_string().parse().unwrap();
    let database_name = "notmydatabase";

    url.set_path(&format!("/{database_name}"));

    let dm = r#"
        datasource db {
            provider = "mysql"
        }
    "#
    .into();

    let error = tok(connection_error(url.as_str(), dm));

    let json_error = serde_json::to_value(error.to_user_facing()).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": "Database `(not available)` does not exist",
        "meta": {
            "database_name": "(not available)",
        },
        "error_code": "P1003"
    });

    assert_eq!(json_error, expected);
}

#[test_connector(tags(Postgres))]
fn bad_datasource_url_and_provider_combinations_must_return_a_proper_error(api: TestApi) {
    let dm = r#"
        datasource db {
            provider = "sqlite"
        }
    "#
    .into();

    let error = tok(connection_error(api.connection_string(), dm));

    let json_error = serde_json::to_value(error.to_user_facing()).unwrap();

    let err_message: String = json_error["message"].as_str().unwrap().into();

    assert!(
        err_message.contains("the URL must start with the protocol `file:`"),
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

#[test_connector(tags(Mysql8))]
fn connections_to_system_databases_must_be_rejected(api: TestApi) {
    let names = &["", "mysql", "sys", "performance_schema"];
    for name in names {
        let mut url: url::Url = api.connection_string().parse().unwrap();
        url.set_path(name);

        let dm = r#"
            datasource db {
                provider = "mysql"
            }
        "#
        .into();

        // "mysql" is the default in Quaint.
        let name = if name == &"" { "mysql" } else { name };

        let error = tok(connection_error(url.as_str(), dm));
        let json_error = serde_json::to_value(error.to_user_facing()).unwrap();

        let expected = json!({
            "is_panic": false,
            "message": format!("The `{name}` database is a system database, it should not be altered with prisma migrate. Please connect to another database."),
            "meta": {
                "database_name": name,
            },
            "error_code": "P3004",
        });

        assert_eq!(json_error, expected);
    }
}

#[test_connector(tags(Sqlite))]
fn datamodel_parser_errors_must_return_a_known_error(api: TestApi) {
    let bad_dm = r#"
        model Test {
            id Float @id
            post Post[]
        }
    "#;

    let error = api.schema_push_w_datasource(bad_dm).send_unwrap_err().to_user_facing();

    let expected_msg = "\u{1b}[1;91merror\u{1b}[0m: \u{1b}[1mType \"Post\" is neither a built-in type, nor refers to another model, composite type, or enum.\u{1b}[0m\n  \u{1b}[1;94m-->\u{1b}[0m  \u{1b}[4mschema.prisma:10\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n\u{1b}[1;94m 9 | \u{1b}[0m            id Float @id\n\u{1b}[1;94m10 | \u{1b}[0m            post \u{1b}[1;91mPost\u{1b}[0m[]\n\u{1b}[1;94m   | \u{1b}[0m\n";

    let expected_error = user_facing_errors::Error::from(user_facing_errors::KnownError {
        error_code: std::borrow::Cow::Borrowed("P1012"),
        message: expected_msg.into(),
        meta: serde_json::json!({ "full_error": expected_msg }),
    });

    assert_eq!(error, expected_error);
}

#[test_connector(exclude(CockroachDb, Sqlite))]
fn unique_constraint_errors_in_migrations_must_return_a_known_error(api: TestApi) {
    let dm = r#"
        model Fruit {
            id   Int @id @default(autoincrement())
            name String
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let insert = Insert::multi_into(api.render_table_name("Fruit"), ["name"])
        .values(("banana",))
        .values(("apple",))
        .values(("banana",));

    api.query(insert.into());

    let dm2 = r#"
        model Fruit {
            id   Int @id @default(autoincrement())
            name String @unique
        }
    "#;

    let res = api
        .schema_push_w_datasource(dm2)
        .force(true)
        .migration_id(Some("the-migration"))
        .send_unwrap_err()
        .to_user_facing();

    let json_error = serde_json::to_value(&res).unwrap();

    let expected_msg = if api.is_vitess() {
        "Unique constraint failed on the (not available)"
    } else if api.is_mysql() || api.is_mssql() {
        "Unique constraint failed on the constraint: `Fruit_name_key`"
    } else {
        "Unique constraint failed on the fields: (`name`)"
    };

    let expected_target = if api.is_vitess() {
        serde_json::Value::Null
    } else if api.is_mysql() || api.is_mssql() {
        json!("Fruit_name_key")
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
}

#[test_connector(tags(Mysql56))]
fn json_fields_must_be_rejected_on_mysql_5_6(api: TestApi) {
    let dm = r#"
        model Test {
            id Int @id
            j Json
        }
        "#;

    api.ensure_connection_validity().unwrap();

    let result = api
        .schema_push_w_datasource(dm)
        .send_unwrap_err()
        .to_user_facing()
        .unwrap_known();

    assert_eq!(result.error_code, "P1015");
    assert!(
        result
            .message
            .contains("Your Prisma schema is using features that are not supported for the version of the database")
    );
    assert!(
        result
            .message
            .contains("- The `Json` data type used in Test.j is not supported on MySQL 5.6.\n")
    );
}

#[tokio::test]
async fn connection_string_problems_give_a_nice_error() {
    let providers = [
        ("mysql", "mysql://root:password-with-#@localhost:3306/database"),
        (
            "postgresql",
            "postgresql://root:password-with-#@localhost:5432/postgres",
        ),
        ("sqlserver", "sqlserver://root:password-with-#@localhost:5432/postgres"),
    ];

    for (provider, url) in providers {
        eprintln!("Provider: {provider}");
        let dm = formatdoc!(
            r#"
                datasource db {{
                  provider = "{}"
                }}
        "#,
            provider,
        );

        let mut api =
            schema_core::schema_api_without_extensions(Some(dm.clone()), DatasourceUrls::from_url(url), None).unwrap();

        let error = api
            .ensure_connection_validity(EnsureConnectionValidityParams {
                datasource: DatasourceParam::Schema(SchemasContainer {
                    files: vec![SchemaContainer {
                        path: "schema.prisma".to_string(),
                        content: dm,
                    }],
                }),
            })
            .await
            .unwrap_err();
        api.dispose().await.unwrap();

        let json_error = serde_json::to_value(error.to_user_facing()).unwrap();

        let details = match provider {
            "sqlserver" => {
                indoc!(
                    "Error parsing connection string: Conversion error: invalid digit found in string in database URL.
                    Please refer to the documentation in https://www.prisma.io/docs/reference/database-reference/connection-urls
                    for constructing a correct connection string. In some cases, certain characters must be escaped.
                    Please check the string for any illegal characters.",
                ).replace('\n', " ")
            },
            _ => {
                indoc!(
                    "invalid port number in database URL.
                    Please refer to the documentation in https://www.prisma.io/docs/reference/database-reference/connection-urls
                    for constructing a correct connection string. In some cases, certain characters must be escaped.
                    Please check the string for any illegal characters.",
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

// Failing due to no color output on Windows :(
#[cfg(unix)]
#[tokio::test]
async fn bad_connection_string_in_datamodel_returns_nice_error() {
    let schema = indoc! {r#"
        datasource db {
          provider = "postgresql"
        }

        generator client {
          provider = "prisma-client"
        }
    "#};

    let error = match schema_core::schema_api_without_extensions(
        Some(schema.to_string()),
        DatasourceUrls::from_url(
            "sqlserver:/localhost:1433;database=prisma-demo;user=SA;password=Pr1sm4_Pr1sm4;trustServerCertificate=true;encrypt=true",
        ),
        None,
    ) {
        Ok(_) => panic!("Did not error"),
        Err(e) => e,
    };

    let json_error = serde_json::to_value(error.to_user_facing()).unwrap();

    let expected_json_error = json!({
        "is_panic": false,
        "message": "\u{1b}[1;91merror\u{1b}[0m: \u{1b}[1mError validating datasource `db`: the URL must start with the protocol `postgresql://` or `postgres://`.\u{1b}[0m\n  \u{1b}[1;94m-->\u{1b}[0m  \u{1b}[4mschema.prisma:3\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n\u{1b}[1;94m 2 | \u{1b}[0m  provider = \"postgresql\"\n\u{1b}[1;94m 3 | \u{1b}[0m  url      = \u{1b}[1;91m\"sqlserver:/localhost:1433;database=prisma-demo;user=SA;password=Pr1sm4_Pr1sm4;trustServerCertificate=true;encrypt=true\"\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n",
        "meta": {
            "full_error": "\u{1b}[1;91merror\u{1b}[0m: \u{1b}[1mError validating datasource `db`: the URL must start with the protocol `postgresql://` or `postgres://`.\u{1b}[0m\n  \u{1b}[1;94m-->\u{1b}[0m  \u{1b}[4mschema.prisma:3\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n\u{1b}[1;94m 2 | \u{1b}[0m  provider = \"postgresql\"\n\u{1b}[1;94m 3 | \u{1b}[0m  url      = \u{1b}[1;91m\"sqlserver:/localhost:1433;database=prisma-demo;user=SA;password=Pr1sm4_Pr1sm4;trustServerCertificate=true;encrypt=true\"\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n",
        },
        "error_code": "P1012",
    });

    assert_eq!(json_error, expected_json_error);
}
