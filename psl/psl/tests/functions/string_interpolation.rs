use crate::common::*;
use psl::parser_database::ScalarType;

#[test]
fn should_not_remove_whitespace() {
    for provider in [
        None,
        Some(crate::Provider::KingbaseMysql),
        Some(crate::Provider::KingbaseOracle),
    ] {
        let dml = indoc! {r#"
            model User {
              id        Int    @id
              firstName String @default("This is a string with whitespace")
            }
        "#};
        let dml = match provider {
            Some(provider) => crate::with_header(dml, provider, &[]),
            None => dml.to_owned(),
        };

        psl::parse_schema_without_extensions(&dml)
            .unwrap()
            .assert_has_model("User")
            .assert_has_scalar_field("firstName")
            .assert_scalar_type(ScalarType::String)
            .assert_default_value()
            .assert_string("This is a string with whitespace");
    }
}

#[test]
fn should_not_try_to_interpret_comments_in_strings() {
    for provider in [
        None,
        Some(crate::Provider::KingbaseMysql),
        Some(crate::Provider::KingbaseOracle),
    ] {
        let dml = indoc! {r#"
            model User {
              id        Int    @id
              firstName String @default("This is a string with a // Comment")
            }
        "#};
        let dml = match provider {
            Some(provider) => crate::with_header(dml, provider, &[]),
            None => dml.to_owned(),
        };

        psl::parse_schema_without_extensions(&dml)
            .unwrap()
            .assert_has_model("User")
            .assert_has_scalar_field("firstName")
            .assert_scalar_type(ScalarType::String)
            .assert_default_value()
            .assert_string("This is a string with a // Comment");
    }
}
