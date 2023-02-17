use indoc::{formatdoc, indoc};
use query_core::protocol::EngineProtocol;
use serde_json::json;

use crate::context::{PrismaContext, ServerConfig};

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

                model A {{
                  id Int @id @default(autoincrement())
                }}
        "#,
            provider.0,
            provider.1
        );

        let dml = psl::parse_schema(dm).unwrap();

        let mut sc = ServerConfig::default();
        sc.enable_raw_queries = true;

        let error = PrismaContext::new(dml, EngineProtocol::Graphql, sc, None)
            .await
            .unwrap_err();

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
