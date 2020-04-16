extern crate datamodel;
use pretty_assertions::assert_eq;
use std::str;

#[test]
fn test_reformat_model() {
    let input = r#"
        model User { 
            id               Int                   @id 
        }
    "#;

    let expected = r#"model User {
  id Int @id
}"#;

    let mut buf = Vec::new();
    datamodel::ast::reformat::Reformatter::reformat_to(&input, &mut buf, 2);
    let actual = str::from_utf8(&buf).expect("unable to convert to string");
    assert_eq!(expected, actual);
}

#[test]
fn test_reformat_config() {
    let input = r#"
        datasource pg { 
            provider = "postgresql"
            url = "postgresql://"
        }
    "#;

    let expected = r#"datasource pg {
  provider = "postgresql"
  url      = "postgresql://"
}"#;

    let mut buf = Vec::new();
    datamodel::ast::reformat::Reformatter::reformat_to(&input, &mut buf, 2);
    let actual = str::from_utf8(&buf).expect("unable to convert to string");
    assert_eq!(expected, actual);
}

#[test]
fn test_reformat_tabs() {
    let input = r#"
        datasource pg {
            provider\t=\t"postgresql"
            url = "postgresql://"
        }
    "#;

    let expected = r#"datasource pg {
  provider = "postgresql"
  url      = "postgresql://"
}"#;

    let mut buf = Vec::new();
    // replaces \t placeholder with a real tab
    datamodel::ast::reformat::Reformatter::reformat_to(&input.replace("\\t", "\t"), &mut buf, 2);
    let actual = str::from_utf8(&buf).expect("unable to convert to string");
    assert_eq!(expected, actual);
}

#[test]
fn test_floating_doc_comment() {
    let input = r#"
model a {
  one Int
  two Int
  // bs  b[] @relation(references: [a])
  @@id([one, two])
}

/// ajlsdkfkjasflk
// model ok {}"#;

    let _expected = r#"
model a {
  one Int
  two Int
  // bs  b[] @relation(references: [a])
  @@id([one, two])
}

/// ajlsdkfkjasflk
// model ok {}"#;

    let mut buf = Vec::new();
    // replaces \t placeholder with a real tab
    datamodel::ast::reformat::Reformatter::reformat_to(&input.replace("\\t", "\t"), &mut buf, 2);
    // FIXME: This is ignored. See explanation in following test for details on why.
    //    let actual = str::from_utf8(&buf).expect("unable to convert to string");
    //    assert_eq!(expected, actual);
}

#[test]
fn test_floating_doc_comments() {
    let input = r#"
model a {
  one Int
  two Int
  // bs  b[] @relation(references: [a])
  @@id([one, two])
}

// ajlsdkfkjasflk
// ajlsdkfkjasflk
"#;

    let _expected = r#"
model a {
  one Int
  two Int
  // bs  b[] @relation(references: [a])
  @@id([one, two])
}

// ajlsdkfkjasflk
// ajlsdkfkjasflk"#;

    let mut buf = Vec::new();
    // replaces \t placeholder with a real tab
    datamodel::ast::reformat::Reformatter::reformat_to(&input.replace("\\t", "\t"), &mut buf, 2);
    let _actual = str::from_utf8(&buf).expect("unable to convert to string");
    // FIXME: the assertion is ignored for now. We just make sure that the reformatting at least does not crash.
    // FIXME: It's hard to implement this because the reformatting does not operate purely on the AST anymore and goes through dml layer and back.
    // FIXME: This means that the following information gets lost:
    // FIXME: 1. The commented field gets simply assigned to the model. It is not known where it was originally placed.
    // FIXME: 2. The floating comments are not present in the dml representation at all. They get lost.
    //    assert_eq!(expected, actual);
}

#[test]
fn reformatting_enums_must_work() {
    let input = r#"
enum Colors {
  RED
  BLUE
  GREEN
  
  // comment
  ORANGE
}
"#;

    // moving the comment to the top is not ideal. Just want to capture the current behavior in a test.
    let expected = r#"// comment
enum Colors {
  RED
  BLUE
  GREEN
  ORANGE
}"#;

    let mut buf = Vec::new();
    datamodel::ast::reformat::Reformatter::reformat_to(&input, &mut buf, 2);
    let actual = str::from_utf8(&buf).expect("unable to convert to string");
    println!("{}", actual);
    assert_eq!(actual, expected);
}

#[test]
fn reformatting_must_work_when_env_var_is_missing() {
    let input = r#"
        datasource pg { 
            provider = "postgresql"
            url = env("DATABASE_URL")
        }
    "#;

    let expected = r#"datasource pg {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}"#;

    let mut buf = Vec::new();
    datamodel::ast::reformat::Reformatter::reformat_to(&input, &mut buf, 2);
    let actual = str::from_utf8(&buf).expect("unable to convert to string");
    assert_eq!(expected, actual);
}
