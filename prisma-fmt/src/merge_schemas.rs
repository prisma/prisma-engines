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

        let expected = expect![];

        let response = merge_schemas(&request.to_string()).unwrap_err();
        expected.assert_eq(&response);
    }
}
