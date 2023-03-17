use crate::common::*;
use psl::parser_database::ScalarType;

#[test]
fn correctly_handle_server_side_now_function() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          signupDate DateTime @default(now())
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_has_scalar_field("signupDate")
        .assert_scalar_type(ScalarType::DateTime)
        .assert_default_value()
        .assert_now();
}

#[test]
fn correctly_handle_server_side_cuid_function() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          someId String @default(cuid())
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_has_scalar_field("someId")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_cuid();
}

#[test]
fn correctly_handle_server_side_uuid_function() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          someId String @default(uuid())
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_has_scalar_field("someId")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_uuid();
}
