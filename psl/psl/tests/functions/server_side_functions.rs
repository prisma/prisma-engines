use crate::common::*;
use psl::parser_database::ScalarType;

#[test]
fn correctly_handle_server_side_now_function() {
    for provider in [
        None,
        Some(crate::Provider::KingbaseMysql),
        Some(crate::Provider::KingbaseOracle),
    ] {
        let dml = indoc! {r#"
            model User {
              id Int @id
              signupDate DateTime @default(now())
            }
        "#};
        let dml = match provider {
            Some(provider) => crate::with_header(dml, provider, &[]),
            None => dml.to_owned(),
        };

        psl::parse_schema_without_extensions(&dml)
            .unwrap()
            .assert_has_model("User")
            .assert_has_scalar_field("signupDate")
            .assert_scalar_type(ScalarType::DateTime)
            .assert_default_value()
            .assert_now();
    }
}

#[test]
fn correctly_handle_server_side_cuid_function() {
    for provider in [
        None,
        Some(crate::Provider::KingbaseMysql),
        Some(crate::Provider::KingbaseOracle),
    ] {
        let dml = indoc! {r#"
            model User {
              id Int @id
              someId String @default(cuid())
            }
        "#};
        let dml = match provider {
            Some(provider) => crate::with_header(dml, provider, &[]),
            None => dml.to_owned(),
        };

        psl::parse_schema_without_extensions(&dml)
            .unwrap()
            .assert_has_model("User")
            .assert_has_scalar_field("someId")
            .assert_scalar_type(ScalarType::String)
            .assert_default_value()
            .assert_cuid();
    }
}

#[test]
fn correctly_handle_server_side_uuid_function() {
    for provider in [
        None,
        Some(crate::Provider::KingbaseMysql),
        Some(crate::Provider::KingbaseOracle),
    ] {
        let dml = indoc! {r#"
            model User {
              id Int @id
              someId String @default(uuid())
            }
        "#};
        let dml = match provider {
            Some(provider) => crate::with_header(dml, provider, &[]),
            None => dml.to_owned(),
        };

        psl::parse_schema_without_extensions(&dml)
            .unwrap()
            .assert_has_model("User")
            .assert_has_scalar_field("someId")
            .assert_scalar_type(ScalarType::String)
            .assert_default_value()
            .assert_uuid();
    }
}
