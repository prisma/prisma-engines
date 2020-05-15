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

    assert_reformat(input, expected);
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

    assert_reformat(input, expected);
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

    assert_reformat(&input.replace("\\t", "\t"), expected);
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

    // TODO: that the inner comment is moved to the top is not ideal
    let expected = r#"model a {
  // bs  b[] @relation(references: [a])
  one Int
  two Int
  @@id([one, two])
}
/// ajlsdkfkjasflk
// model ok {}
"#;

    assert_reformat(input, expected);
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

    // TODO: that the inner comment is moved to the top is not ideal
    let expected = r#"model a {
  // bs  b[] @relation(references: [a])
  one Int
  two Int
  @@id([one, two])
}
// ajlsdkfkjasflk
// ajlsdkfkjasflk
"#;

    assert_reformat(input, expected);
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
