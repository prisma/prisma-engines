use crate::common::*;
use psl::dml::{self, PrismaValue, ScalarType};

#[test]
fn should_not_remove_whitespace() {
    let dml = r#"
    model User {
        id Int @id
        firstName String @default("This is a string with whitespace")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_scalar_field("firstName")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(dml::DefaultValue::new_single(PrismaValue::String(String::from(
            "This is a string with whitespace",
        ))));
}

#[test]
fn should_not_try_to_interpret_comments_in_strings() {
    let dml = r#"
    model User {
        id Int @id
        firstName String @default("This is a string with a // Comment")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_scalar_field("firstName")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(dml::DefaultValue::new_single(PrismaValue::String(String::from(
            "This is a string with a // Comment",
        ))));
}
