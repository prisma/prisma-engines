use enumflags2::BitFlags;
use indoc::indoc;
use introspection_core::RpcImpl;
use pretty_assertions::assert_eq;
use serde_json::json;
use sql_introspection_connector::SqlIntrospectionConnector;
use user_facing_errors::{common::SchemaParserError, UserFacingError};

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
        let error = SqlIntrospectionConnector::new(provider.1, BitFlags::empty())
            .await
            .unwrap_err();
        let error = error.user_facing_error().cloned().unwrap();
        let error = user_facing_errors::Error::from(error);
        let json_error = serde_json::to_value(&error).unwrap();

        let details = match provider.0 {
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
                    "Error parsing connection string: invalid port number in database URL.
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

#[tokio::test]
async fn bad_connection_string_in_datamodel_returns_nice_error() {
    let schema = r#"
    datasource db {
        provider = "postgresql"
        url      = "sqlserver:/localhost:1433;database=prisma-demo;user=SA;password=Pr1sm4_Pr1sm4;trustServerCertificate=true;encrypt=true"
    }

    generator client {
        provider = "prisma-client-js"
    }
    "#;

    let error = RpcImpl::introspect_internal(schema.into(), false).await.unwrap_err();

    let json_error = serde_json::to_value(error).unwrap();

    let expected_json_error = json!({
        "code": 4466,
        "message": "An error happened. Check the data field for details.",
        "data": {
            "is_panic": false,
            "message": "\u{1b}[1;91merror\u{1b}[0m: \u{1b}[1mError validating datasource `db`: the URL must start with the protocol `postgresql://` or `postgres://`.\u{1b}[0m\n  \u{1b}[1;94m-->\u{1b}[0m  \u{1b}[4mschema.prisma:4\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n\u{1b}[1;94m 3 | \u{1b}[0m        provider = \"postgresql\"\n\u{1b}[1;94m 4 | \u{1b}[0m        url      = \u{1b}[1;91m\"sqlserver:/localhost:1433;database=prisma-demo;user=SA;password=Pr1sm4_Pr1sm4;trustServerCertificate=true;encrypt=true\"\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n",
            "meta": {
                "full_error": "\u{1b}[1;91merror\u{1b}[0m: \u{1b}[1mError validating datasource `db`: the URL must start with the protocol `postgresql://` or `postgres://`.\u{1b}[0m\n  \u{1b}[1;94m-->\u{1b}[0m  \u{1b}[4mschema.prisma:4\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n\u{1b}[1;94m 3 | \u{1b}[0m        provider = \"postgresql\"\n\u{1b}[1;94m 4 | \u{1b}[0m        url      = \u{1b}[1;91m\"sqlserver:/localhost:1433;database=prisma-demo;user=SA;password=Pr1sm4_Pr1sm4;trustServerCertificate=true;encrypt=true\"\u{1b}[0m\n\u{1b}[1;94m   | \u{1b}[0m\n",
            },
            "error_code": SchemaParserError::ERROR_CODE,
        }
    });

    assert_eq!(json_error, expected_json_error);
}
