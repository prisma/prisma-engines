use crate::common::*;
use psl::parser_database::ScalarType;

#[test]
fn should_not_remove_whitespace() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String @default("This is a string with whitespace")
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_has_scalar_field("firstName")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_string("This is a string with whitespace");
}

#[test]
fn should_not_try_to_interpret_comments_in_strings() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String @default("This is a string with a // Comment")
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_has_scalar_field("firstName")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_string("This is a string with a // Comment");
}
