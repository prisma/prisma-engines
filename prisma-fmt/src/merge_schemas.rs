use psl::reformat_validated_schema_into_single;
use serde::Deserialize;

use crate::schema_file_input::SchemaFileInput;

#[derive(Debug, Deserialize)]
pub struct MergeSchemasParams {
    schema: SchemaFileInput,
}

pub(crate) fn merge_schemas(params: &str) -> Result<String, String> {
    let params: MergeSchemasParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(serde_err) => {
            panic!("Failed to deserialize MergeSchemasParams: {serde_err}");
        }
    };

    let validated_schema = crate::validate::run(params.schema, false)?;

    let indent_width = 2usize;
    let merged_schema = reformat_validated_schema_into_single(validated_schema, indent_width).unwrap();

    Ok(merged_schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;
    use serde_json::json;

    #[test]
    fn merge_two_valid_schemas_succeeds() {
        let schema = vec![
            (
                "b.prisma",
                r#"
                model B {
                    id String @id
                    a A?
                }
            "#,
            ),
            (
                "a.prisma",
                r#"
                datasource db {
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
        ];

        let request = json!({
            "schema": schema,
        });

        let expected = expect![[r#"
            model B {
              id String @id
              a  A?
            }

            datasource db {
              provider = "postgresql"
              url      = env("DBURL")
            }

            model A {
              id   String @id
              b_id String @unique
              b    B      @relation(fields: [b_id], references: [id])
            }
        "#]];

        let response = merge_schemas(&request.to_string()).unwrap();
        expected.assert_eq(&response);
    }

    #[test]
    fn merge_two_invalid_schemas_panics() {
        let schema = vec![
            (
                "b.prisma",
                r#"
                model B {
                    id String @id
                    a A?
                }
            "#,
            ),
            (
                "a.prisma",
                r#"
                datasource db {
                    provider = "postgresql"
                    url = env("DBURL")
                }

                model A {
                    id String @id
                    b_id String @unique
                }
            "#,
            ),
        ];

        let request = json!({
            "schema": schema,
        });

        let expected = expect![[
            r#"{"error_code":"P1012","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating field `a` in model `B`: The relation field `a` on model `B` is missing an opposite relation field on the model `A`. Either run `prisma format` or add it manually.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mb.prisma:4\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 3 | \u001b[0m                    id String @id\n\u001b[1;94m 4 | \u001b[0m                    \u001b[1;91ma A?\u001b[0m\n\u001b[1;94m 5 | \u001b[0m                }\n\u001b[1;94m   | \u001b[0m\n\nValidation Error Count: 1"}"#
        ]];

        let response = merge_schemas(&request.to_string()).unwrap_err();
        expected.assert_eq(&response);
    }
}
