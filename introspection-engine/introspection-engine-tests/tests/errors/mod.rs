use indoc::formatdoc;
use serde_json::json;
use sql_introspection_connector::SqlIntrospectionConnector;

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
        let error = SqlIntrospectionConnector::new(provider.1).await.unwrap_err();
        let error = error.user_facing_error().map(|e| e.clone()).unwrap();
        let error = user_facing_errors::Error::from(error);
        let json_error = serde_json::to_value(&error).unwrap();

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
