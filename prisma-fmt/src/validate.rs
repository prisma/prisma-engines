use psl::ValidatedSchema;
use serde::Deserialize;
use serde_json::json;
use std::fmt::Write as _;

use crate::schema_file_input::SchemaFileInput;

// this mirrors user_facing_errors::common::SchemaParserError
pub(crate) static SCHEMA_PARSER_ERROR_CODE: &str = "P1012";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ValidateParams {
    prisma_schema: SchemaFileInput,
    #[serde(default)]
    no_color: bool,
}

pub(crate) fn validate(params: &str) -> Result<(), String> {
    let params: ValidateParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(serde_err) => {
            panic!("Failed to deserialize ValidateParams: {serde_err}");
        }
    };

    run(params.prisma_schema, params.no_color)?;
    Ok(())
}

pub fn run(input_schema: SchemaFileInput, no_color: bool) -> Result<ValidatedSchema, String> {
    let sources: Vec<(String, psl::SourceFile)> = input_schema.into();
    let validate_schema = psl::validate_multi_file(&sources);
    let diagnostics = &validate_schema.diagnostics;

    if !diagnostics.has_errors() {
        return Ok(validate_schema);
    }

    // always colorise output regardless of the environment, which is important for Wasm
    colored::control::set_override(!no_color);

    let mut formatted_error = validate_schema.render_own_diagnostics();
    write!(
        formatted_error,
        "\nValidation Error Count: {}",
        diagnostics.errors().len(),
    )
    .unwrap();
    Err(json!({
        "error_code": SCHEMA_PARSER_ERROR_CODE,
        "message": formatted_error,
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn validate_invalid_schema_with_colors() {
        let schema = r#"
            generator js {
            }

            datasøurce yolo {
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"error_code":"P1012","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m            \u001b[1;91mdatasøurce yolo {\u001b[0m\n\u001b[1;94m 6 | \u001b[0m            }\n\u001b[1;94m   | \u001b[0m\n\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:6\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m            datasøurce yolo {\n\u001b[1;94m 6 | \u001b[0m            \u001b[1;91m}\u001b[0m\n\u001b[1;94m 7 | \u001b[0m        \n\u001b[1;94m   | \u001b[0m\n\u001b[1;91merror\u001b[0m: \u001b[1mArgument \"provider\" is missing in generator block \"js\".\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:2\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 1 | \u001b[0m\n\u001b[1;94m 2 | \u001b[0m            \u001b[1;91mgenerator js {\u001b[0m\n\u001b[1;94m 3 | \u001b[0m            }\n\u001b[1;94m   | \u001b[0m\n\nValidation Error Count: 3"}"#
        ]];

        let response = validate(&request.to_string()).unwrap_err();
        expected.assert_eq(&response);
    }

    #[test]
    fn validate_missing_env_var() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST")
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        validate(&request.to_string()).unwrap();
    }

    #[test]
    fn validate_direct_url_direct_empty() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("DBURL")
                directUrl = ""
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        validate(&request.to_string()).unwrap();
    }

    #[test]
    fn validate_multiple_files() {
        let schema = vec![
            (
                "a.prisma",
                r#"
                datasource thedb {
                    provider = "postgresql"
                    url = env("DBURL")
                }

                model A {
                    id String @id
                    b_id String @unique
                    b B @relation(fields: [b_id], references: [id])
                }
            "#,
            ),
            (
                "b.prisma",
                r#"
                model B {
                    id String @id
                    a A?
                }
            "#,
            ),
        ];

        let request = json!({
            "prismaSchema": schema,
        });

        validate(&request.to_string()).unwrap();
    }

    #[test]
    fn validate_multiple_files_error() {
        let schema = vec![
            (
                "a.prisma",
                r#"
                datasource thedb {
                    provider = "postgresql"
                    url = env("DBURL")
                }

                model A {
                    id String @id
                    b_id String @unique
                    b B @relation(fields: [b_id], references: [id])
                }
            "#,
            ),
            (
                "b.prisma",
                r#"
                model B {
                    id String @id
                    a A
                }
            "#,
            ),
        ];

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"error_code":"P1012","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError parsing attribute \"@relation\": The relation field `a` on Model `B` is required. This is not valid because it's not possible to enforce this constraint on the database level. Please change the field type from `A` to `A?` to fix this.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mb.prisma:4\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 3 | \u001b[0m                    id String @id\n\u001b[1;94m 4 | \u001b[0m                    \u001b[1;91ma A\u001b[0m\n\u001b[1;94m 5 | \u001b[0m                }\n\u001b[1;94m   | \u001b[0m\n\nValidation Error Count: 1"}"#
        ]];

        let response = validate(&request.to_string()).unwrap_err();
        expected.assert_eq(&response);
    }

    #[test]
    fn validate_using_both_relation_mode_and_referential_integrity() {
        let schema = r#"
          datasource db {
              provider = "sqlite"
              url = "sqlite"
              relationMode = "prisma"
              referentialIntegrity = "foreignKeys"
          }
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"error_code":"P1012","message":"\u001b[1;91merror\u001b[0m: \u001b[1mThe `referentialIntegrity` and `relationMode` attributes cannot be used together. Please use only `relationMode` instead.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:6\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m              relationMode = \"prisma\"\n\u001b[1;94m 6 | \u001b[0m              \u001b[1;91mreferentialIntegrity = \"foreignKeys\"\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\nValidation Error Count: 1"}"#
        ]];

        let response = validate(&request.to_string()).unwrap_err();
        expected.assert_eq(&response);
    }
}
