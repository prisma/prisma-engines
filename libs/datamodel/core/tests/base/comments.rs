use crate::common::*;
use datamodel::common::ScalarType;

#[test]
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

#[test]
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
