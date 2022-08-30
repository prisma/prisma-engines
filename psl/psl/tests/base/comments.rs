use crate::common::*;

#[test]
fn comments_must_work_in_models() {
    let dml = r#"
    /// comment 1
    model User { /// comment 2
        id Int @id
        firstName String /// comment 3
        /// comment 4
        lastName String /// comment 5
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User").assert_with_documentation("comment 1");
    user_model
        .assert_has_scalar_field("firstName")
        .assert_with_documentation("comment 3");
    user_model
        .assert_has_scalar_field("lastName")
        .assert_with_documentation("comment 4\ncomment 5");
}

#[test]
fn free_floating_doc_comments_must_work_in_models() {
    let dml = r#"
    model User {
        id Int @id
        firstName String
        /// documentation comment
    }
    "#;

    assert_valid(dml);
}

#[test]
fn free_floating_doc_comments_must_work_in_enums() {
    let dml = r#"
    enum Role {
      USER
      /// documentation comment
    }"#;

    assert_valid(dml);
}

#[test]
fn doc_comments_must_work_on_block_attributes() {
    let dml = r#"
    model Blog {
      id1 Int
      id2 Int
      @@id([id1, id2]) /// Documentation comment block attribute
    }"#;

    assert_valid(dml);
}

#[test]
fn comments_must_work_on_block_attributes() {
    let dml = r#"
    model Blog {
      id1 Int
      id2 Int
      @@id([id1, id2]) // Documentation comment block attribute
    }"#;

    assert_valid(dml);
}

#[test]
fn comments_must_work_in_enums() {
    let dml = r#"
    // Line 1
    // Line 2
    /// Documentation Comment Enum
    enum Role {
      // Comment above
      /// Documentation Comment Enum Value 1
      USER // Comment on the side
      // Comment below
      PIZZAIOLO /// they make the pizza
    }"#;

    let schema = parse(dml);
    let role_enum = schema
        .assert_has_enum("Role")
        .assert_with_documentation("Documentation Comment Enum");
    role_enum
        .assert_has_value("USER")
        .assert_with_documentation("Documentation Comment Enum Value 1");
    role_enum
        .assert_has_value("PIZZAIOLO")
        .assert_with_documentation("they make the pizza");
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
    user_model
        .assert_has_scalar_field("id")
        .assert_base_type(&ScalarType::Int);
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
    user_model
        .assert_has_scalar_field("id")
        .assert_base_type(&ScalarType::Int);
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

    assert_valid(dml);
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

    assert_valid(dml);
}

#[test]
fn two_slash_comments_should_not_lead_to_empty_comments() {
    let dml = r#"
    // two slash comment
    model User2 {
        id        String    @id @default(uuid())
    }"#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User2");
    assert_eq!(user_model.documentation, None);
}
