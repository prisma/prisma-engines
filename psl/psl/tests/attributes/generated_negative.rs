use crate::{Provider, common::*, with_header};

#[test]
fn should_fail_without_preview_feature() {
    let dml = indoc! {r#"
        model User {
          id   Int    @id
          calc Int?   @generated("id * 2")
        }
    "#};

    // No "generatedColumns" in preview features
    let error = parse_unwrap_err(&with_header(dml, Provider::Postgres, &[]));

    assert!(error.contains("preview feature"));
    assert!(error.contains("generatedColumns"));
}

#[test]
fn should_fail_without_expression_argument() {
    let dml = indoc! {r#"
        model User {
          id   Int    @id
          name String @generated
        }
    "#};

    let error = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["generatedColumns"]));

    assert!(error.contains("@generated"));
    assert!(error.contains("Argument"));
}

#[test]
fn should_fail_with_non_string_argument() {
    let dml = indoc! {r#"
        model User {
          id   Int @id
          calc Int @generated(42)
        }
    "#};

    let error = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["generatedColumns"]));

    assert!(error.contains("@generated"));
    assert!(error.contains("string"));
}

#[test]
fn should_fail_when_combined_with_default() {
    let dml = indoc! {r#"
        model User {
          id   Int    @id
          calc Int?   @generated("id * 2") @default(0)
        }
    "#};

    let error = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["generatedColumns"]));

    assert!(error.contains("@generated"));
    assert!(error.contains("@default"));
}

#[test]
fn should_fail_when_combined_with_updated_at() {
    let dml = indoc! {r#"
        model User {
          id   Int      @id
          ts   DateTime @generated("now()") @updatedAt
        }
    "#};

    let error = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["generatedColumns"]));

    assert!(error.contains("@generated"));
    assert!(error.contains("@updatedAt"));
}

#[test]
fn should_fail_when_combined_with_id() {
    let dml = indoc! {r#"
        model User {
          id   Int @id @generated("1")
          name String
        }
    "#};

    let error = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["generatedColumns"]));

    assert!(error.contains("@generated"));
    assert!(error.contains("@id"));
}

#[test]
fn should_fail_on_list_field() {
    let dml = indoc! {r#"
        model User {
          id    Int   @id
          tags  Int[] @generated("ARRAY[1,2,3]")
        }
    "#};

    let error = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["generatedColumns"]));

    assert!(error.contains("@generated"));
    assert!(error.contains("list"));
}

#[test]
fn should_fail_on_unsupported_connector() {
    let dml = indoc! {r#"
        model User {
          id   Int  @id
          calc Int? @generated("id * 2")
        }
    "#};

    let error = parse_unwrap_err(&with_header(dml, Provider::Mysql, &["generatedColumns"]));

    assert!(error.contains("@generated"));
    assert!(error.contains("not supported"));
}
