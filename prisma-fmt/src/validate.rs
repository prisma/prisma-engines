pub(crate) fn run(input_schema: &str) -> Result<(), String> {
    let validate_schema = psl::validate(input_schema.into());
    let diagnostics = &validate_schema.diagnostics;

    if !diagnostics.has_errors() {
        return Ok(());
    }

    use std::fmt::Write as _;
    let mut formatted_error = diagnostics.to_pretty_string("schema.prisma", input_schema);
    write!(
        formatted_error,
        "\nValidation Error Count: {}",
        diagnostics.errors().len(),
    )
    .unwrap();
    Err(formatted_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn validate_invalid_schema() {
        let schema = r#"
            generator js {
            }

            datasøurce yolo {
            }
        "#;

        let expected = expect![[r#"
            [1;91merror[0m: [1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.[0m
              [1;94m-->[0m  [4mschema.prisma:5[0m
            [1;94m   | [0m
            [1;94m 4 | [0m
            [1;94m 5 | [0m            [1;91mdatasøurce yolo {[0m
            [1;94m 6 | [0m            }
            [1;94m   | [0m
            [1;91merror[0m: [1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.[0m
              [1;94m-->[0m  [4mschema.prisma:6[0m
            [1;94m   | [0m
            [1;94m 5 | [0m            datasøurce yolo {
            [1;94m 6 | [0m            [1;91m}[0m
            [1;94m 7 | [0m        
            [1;94m   | [0m
            [1;91merror[0m: [1mArgument "provider" is missing in generator block "js".[0m
              [1;94m-->[0m  [4mschema.prisma:2[0m
            [1;94m   | [0m
            [1;94m 1 | [0m
            [1;94m 2 | [0m            [1;91mgenerator js {[0m
            [1;94m 3 | [0m            }
            [1;94m   | [0m

            Validation Error Count: 3"#]];

        let response = run(schema).unwrap_err();
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

        run(schema).unwrap();
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

        run(schema).unwrap();
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

        let expected = expect![[r#"
            [1;91merror[0m: [1mThe `referentialIntegrity` and `relationMode` attributes cannot be used together. Please use only `relationMode` instead.[0m
              [1;94m-->[0m  [4mschema.prisma:6[0m
            [1;94m   | [0m
            [1;94m 5 | [0m              relationMode = "prisma"
            [1;94m 6 | [0m              [1;91mreferentialIntegrity = "foreignKeys"[0m
            [1;94m 7 | [0m          }
            [1;94m   | [0m

            Validation Error Count: 1"#]];
        let response = run(schema).unwrap_err();
        expected.assert_eq(&response);
    }
}
