use crate::common::*;
use psl::parser_database::ScalarType;
use psl::schema_ast::ast::WithDocumentation;

#[test]
fn comments_must_work_in_models() {
    let dml = indoc! {r#"
        /// comment 1
        model User { /// comment 2
          id Int @id
          firstName String /// comment 3
          /// comment 4
          lastName String /// comment 5
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();

    let user_model = schema.assert_has_model("User");
    user_model.assert_with_documentation("comment 1");

    user_model
        .assert_has_scalar_field("firstName")
        .assert_with_documentation("comment 3");

    user_model
        .assert_has_scalar_field("lastName")
        .assert_with_documentation("comment 4\ncomment 5");
}

#[test]
fn free_floating_doc_comments_must_work_in_models() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String
          /// documentation comment
        }
    "#};

    assert_valid(dml);
}

#[test]
fn free_floating_doc_comments_must_work_in_enums() {
    let dml = indoc! {r#"
        enum Role {
          USER
          /// documentation comment
        }
    "#};

    assert_valid(dml);
}

#[test]
fn doc_comments_must_work_on_block_attributes() {
    let dml = indoc! {r#"
        model Blog {
          id1 Int
          id2 Int
          @@id([id1, id2]) /// Documentation comment block attribute
        }
    "#};

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

    let schema = psl::parse_schema(dml).unwrap();
    let role_enum = schema.db.find_enum("Role").unwrap();
    assert_eq!(role_enum.ast_enum().documentation(), Some("Documentation Comment Enum"));
    let vals: Vec<(_, _)> = role_enum.values().map(|v| (v.name(), v.documentation())).collect();
    assert_eq!(
        vals,
        &[
            ("USER", Some("Documentation Comment Enum Value 1")),
            ("PIZZAIOLO", Some("they make the pizza",))
        ]
    );
}

#[test]
fn accept_a_comment_at_the_end() {
    let dml = r#"
    model User {
        id Int @id
    }
    // This is a comment"#;

    let schema = psl::parse_schema(dml).unwrap();
    let user_model = schema.assert_has_model("User");

    user_model
        .assert_has_scalar_field("id")
        .assert_scalar_type(ScalarType::Int);
}

#[test]
fn accept_a_doc_comment_at_the_end() {
    let dml = r#"
    model User {
        id Int @id
    }
    /// This is a doc comment"#;

    let schema = psl::parse_schema(dml).unwrap();
    let user_model = schema.assert_has_model("User");

    user_model
        .assert_has_scalar_field("id")
        .assert_scalar_type(ScalarType::Int);
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

    let schema = psl::parse_schema(dml).unwrap();
    let user_model = schema.assert_has_model("User2");

    assert_eq!(user_model.ast_model().documentation(), None);
}
