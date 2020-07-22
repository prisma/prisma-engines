extern crate datamodel;
use pretty_assertions::assert_eq;

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

#[test]
fn back_relation_fields_and_directive_must_be_added_even_when_directive_is_missing() {
    let input = r#"model User {
  id Int @id
  post Post
}

model Post {
  id Int @id
}
"#;

    let expected = r#"model User {
  id     Int  @id
  post   Post @relation(fields: [postId], references: [id])
  postId Int?
}

model Post {
  id   Int    @id
  User User[]
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn back_relation_fields_missing_directives_should_not_add_directives_multiple_times() {
    let input = r#"model User {
  id Int @id
  post Post
}

model Post {
  id Int @id
}

model Cat {
  id Int @id
  post Post
}
"#;

    let expected = r#"model User {
  id     Int  @id
  post   Post @relation(fields: [postId], references: [id])
  postId Int?
}

model Post {
  id   Int    @id
  User User[]
  Cat  Cat[]
}

model Cat {
  id     Int  @id
  post   Post @relation(fields: [postId], references: [id])
  postId Int?
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn back_relations_must_be_added_when_directive_is_present_with_no_arguments() {
    let input = r#"model User {
  id Int @id
  post Post @relation
}

model Post {
  id Int @id
}
"#;

    let expected = r#"model User {
  id     Int  @id
  post   Post @relation(fields: [postId], references: [id])
  postId Int?
}

model Post {
  id   Int    @id
  User User[]
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn back_relations_must_be_added_when_directive_is_present_with_only_one_argument() {
    let input = r#"model User {
  id Int @id
  post Post @relation(fields: [postId])
}

model Post {
  id Int @id
}
"#;

    let expected = r#"model User {
  id     Int  @id
  post   Post @relation(fields: [postId], references: [id])
  postId Int?
}

model Post {
  id   Int    @id
  User User[]
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn back_relations_must_be_added_when_directive_is_present_with_both_arguments() {
    let input = r#"model User {
  id Int @id
  post Post @relation(fields: [postId], references: [id])
}

model Post {
  id Int @id
}
"#;

    let expected = r#"model User {
  id     Int  @id
  post   Post @relation(fields: [postId], references: [id])
  postId Int?
}

model Post {
  id   Int    @id
  User User[]
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn scalar_field_and_directive_must_be_added_even_when_directive_is_missing_and_both_relation_fields_present() {
    let input = r#"model User {
  id Int @id
  post Post
}

model Post {
  id Int @id
  User User[]
}
"#;

    let expected = r#"model User {
  id     Int  @id
  post   Post @relation(fields: [postId], references: [id])
  postId Int?
}

model Post {
  id   Int    @id
  User User[]
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn scalar_field_and_directive_must_be_added_even_when_directive_is_missing_and_only_one_relation_fields_present() {
    let input = r#"model User {
  id Int @id
}

model Post {
  id Int @id
  User User[]
}

model Cat {
  id Int @id
  post Post
}
"#;

    let expected = r#"model User {
  id     Int   @id
  Post   Post? @relation(fields: [postId], references: [id])
  postId Int?
}

model Post {
  id   Int    @id
  User User[]
  Cat  Cat[]
}

model Cat {
  id     Int  @id
  post   Post @relation(fields: [postId], references: [id])
  postId Int?
}
"#;

    assert_reformat(input, expected);
}

#[test]
fn back_relations_must_be_added_even_when_directive_is_missing_for_one_to_one() {
    let input = r#"model User {
  id     Int   @id
  Post   Post?
}

model Post {
  id   Int    @id
  User User
}
"#;
    let expected = r#"model User {
  id   Int   @id
  Post Post?
}

model Post {
  id     Int  @id
  User   User @relation(fields: [userId], references: [id])
  userId Int
}
"#;
    assert_reformat(input, expected);
}

#[test]
fn back_relations_and_directive_must_be_added_even_when_directive_is_missing_for_one_to_many() {
    let input = r#"model User {
  id     Int   @id
  Post   Post
}

model Post {
  id   Int    @id
  User User[]
}
"#;
    let expected = r#"model User {
  id     Int  @id
  Post   Post @relation(fields: [postId], references: [id])
  postId Int
}

model Post {
  id   Int    @id
  User User[]
}
"#;
    assert_reformat(input, expected);
}

#[test]
fn relation_directive_must_be_added_for_many_to_many() {
    let input = r#"model User {
  id     Int   @id
  Post   Post[]
}

model Post {
  id   Int    @id
  User User[]
}
"#;
    let expected = r#"model User {
  id   Int    @id
  Post Post[] @relation(references: [id])
}

model Post {
  id   Int    @id
  User User[] @relation(references: [id])
}
"#;
    assert_reformat(input, expected);
}

#[test]
fn relation_directive_must_be_added_for_many_to_many_with_multi_field_id() {
    let input = r#"model User {
  id     Int   @id
  Post   Post[]
}

model Post {
  name String
  position Int
  User  User[]
  @@id([name, position])
}
"#;
    let expected = r#"model User {
  id     Int    @id
  Post   Post[] @relation(references: [name, position])
}

model Post {
  name     String
  position Int
  User     User[] @relation(references: [id])
  @@id([name, position])
}
"#;
    assert_reformat(input, expected);
}

#[test]
fn back_relations_must_be_added_even_when_env_vars_are_missing() {
    // missing env vars led to errors in datamodel validation. A successful validation is prerequisite to find missing back relation fields though.
    // I changed the Reformatter to ignore env var errors.
    let input = r#"
datasource db {
  provider = "sqlite"
  url      = env("DATABASE_URL")
}

model Blog {
  id    Int    @id
  posts Post[]
}

model Post {
  id Int   @id
}
"#;

    let expected = r#"datasource db {
  provider = "sqlite"
  url      = env("DATABASE_URL")
}

model Blog {
  id    Int    @id
  posts Post[]
}

model Post {
  id     Int   @id
  Blog   Blog? @relation(fields: [blogId], references: [id])
  blogId Int?
}
"#;

    assert_reformat(input, expected);
}

#[test]
#[ignore]
fn must_add_relation_directive_to_an_existing_field() {
    let input = r#"
    model Blog {
      id    Int     @id
      posts Post[]
    }
    
    model Post {
      id     Int   @id
      Blog   Blog? @relation(fields: [blogId])
      blogId Int?
    }    
    "#;

    let expected = r#"model Blog {
  id    Int    @id
  posts Post[]
}

model Post {
  id     Int   @id
  Blog   Blog? @relation(fields: [blogId], references: [id])
  blogId Int?
}
"#;
    assert_reformat(input, expected);
}

fn assert_reformat(schema: &str, expected_result: &str) {
    println!("schema: {:?}", schema);
    let result = datamodel::ast::reformat::Reformatter::new(&schema).reformat_to_string();
    println!("result: {}", result);
    assert_eq!(result, expected_result);
}
