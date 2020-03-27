use crate::common::*;
use datamodel::common::ScalarType;

// TODO: figure out if this is a feature we want (the weird definition of `firstName`). I don't think so.
#[test]
#[ignore]
fn parse_comments_without_crasing_or_loosing_info() {
    let dml = r#"
    // This is a comment
    model User { // This is a comment
        id Int @id
        firstName // Also a comment.
        String
        // This is also a comment
        lastName String // This is a comment
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_is_embedded(false);
    user_model.assert_has_field("id").assert_base_type(&ScalarType::Int);
    user_model
        .assert_has_field("firstName")
        .assert_base_type(&ScalarType::String);
    user_model
        .assert_has_field("lastName")
        .assert_base_type(&ScalarType::String);
}

// TODO: figure out if this is a feature we want. I don't think so.
#[test]
#[ignore]
fn accept_a_comment_at_the_end() {
    let dml = r#"
    model User {
        id Int @id
    }
    // This is a comment"#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_is_embedded(false);
    user_model.assert_has_field("id").assert_base_type(&ScalarType::Int);
}

#[test]
fn accept_a_doc_comment_at_the_end() {
    let dml = r#"
    model User {
        id Int @id
    }
    /// This is a doc comment"#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_is_embedded(false);
    user_model.assert_has_field("id").assert_base_type(&ScalarType::Int);
}

#[test]
fn comments_must_work_in_enums() {
    let dml = r#"
    // Line 1
    // Line 2
    enum Role {
      // Comment above
      USER // Comment on the side
      // Comment below
    }"#;

    // must not crash
    let _ = parse(dml);
}

#[test]
fn comments_in_a_generator_must_work() {
    let dml = r#"
    generator gen {
        provider  = "predefined-generator"
        platforms = ["darwin"]
        // platforms is deprecated
    }
    "#;

    // must not crash
    let _ = parse(dml);
}

#[test]
fn comments_in_a_datasource_must_work() {
    let dml = r#"
        datasource db {
            provider = "postgresql"
            // Like, postgresql://user:password@localhost:5432/database/schema
            url      = env("PARCEL_PG_URL")
        }
    "#;
    std::env::set_var(
        "PARCEL_PG_URL",
        "postgresql://user:password@localhost:5432/database/schema",
    );

    // must not crash
    let _ = parse(dml);
}
