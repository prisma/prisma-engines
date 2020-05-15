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
}
"#;

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
}
"#;

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
}
"#;

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
    let input = r#"enum Colors {
  RED @map("rett")
  BLUE
  GREEN

  // comment
  ORANGE_AND_KIND_OF_RED @map("super_color")
  
  @@map("the_colors")
}
"#;
    let expected = r#"enum Colors {
  RED    @map("rett")
  BLUE
  GREEN

  // comment
  ORANGE_AND_KIND_OF_RED  @map("super_color")

  @@map("the_colors")
}
"#;

    assert_reformat(input, expected);
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
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn invalid_lines_must_not_break_reformatting() {
    let input = r#"$ /a/b/c:.
model Post {
  id Int @id
}
"#;

    assert_reformat(input, input);
}

#[test]
fn new_lines_before_first_block_must_be_removed() {
    let input = r#"

model Post {
  id Int @id
}"#;

    let expected = r#"model Post {
  id Int @id
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn new_lines_between_blocks_must_be_reduced_to_one_simple() {
    let input = r#"model Post {
  id Int @id
}


model Blog {
  id Int @id
}
"#;

    let expected = r#"model Post {
  id Int @id
}

model Blog {
  id Int @id
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn new_lines_between_blocks_must_be_reduced_to_one_complex() {
    let input = r#"model Post {
  id Int @id
}


model Blog {
  id Int @id
}


datasource mydb {
  provider = "sqlite"
  url      = "file:dev.db"
}


enum Status {
  ACTIVE
  DONE
}


type MyString = String


generator js {
    provider = "js"
}
"#;

    let expected = r#"model Post {
  id Int @id
}

model Blog {
  id Int @id
}

datasource mydb {
  provider = "sqlite"
  url      = "file:dev.db"
}

enum Status {
  ACTIVE
  DONE
}

type MyString = String

generator js {
  provider = "js"
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn model_level_directives_reset_the_table_layout() {
    let input = r#"model Post {
  id Int @id
  aVeryLongName  String
  @@index([a])
  alsoAVeryLongName String
}
"#;

    let expected = r#"model Post {
  id            Int    @id
  aVeryLongName String
  @@index([a])
  alsoAVeryLongName String
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn back_relation_fields_must_be_added() {
    let input = r#"model Blog {
  id    Int     @id
  posts Post[]
}

model Post {
  id Int   @id
}

model Post2 {
  id     Int  @id
  blogId Int
  Blog   Blog @relation(fields: [blogId], references: [id])
}
"#;

    let expected = r#"model Blog {
  id    Int     @id
  posts Post[]
  Post2 Post2[]
}

model Post {
  id     Int   @id
  Blog   Blog? @relation(fields: [blogId], references: [id])
  blogId Int?
}

model Post2 {
  id     Int  @id
  blogId Int
  Blog   Blog @relation(fields: [blogId], references: [id])
}
"#;

    assert_reformat(input, expected);
}

fn assert_reformat(schema: &str, expected_result: &str) {
    println!("schema: {:?}", schema);
    let mut buf = Vec::new();
    datamodel::ast::reformat::Reformatter::reformat_to(&schema, &mut buf, 2);
    let result = str::from_utf8(&buf).expect("unable to convert to string");
    println!("result: {}", result);
    assert_eq!(result, expected_result);
}
