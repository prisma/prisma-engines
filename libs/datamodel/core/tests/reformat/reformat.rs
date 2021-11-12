use crate::common::*;
use datamodel::ast::reformat::Reformatter;
use indoc::indoc;

#[test]
fn must_add_new_line_to_end_of_schema() {
    let input = r#"// a comment"#;

    let expected = expect![[r#"
        // a comment
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn test_reformat_model_simple() {
    let input = indoc! {r#"
        model User {
          id               Int                   @id
        }
    "#};

    let expected = expect![[r#"
        model User {
          id Int @id
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn test_reformat_model_complex() {
    let input = indoc! {r#"
        /// model doc comment
        model User {
          id Int @id // doc comment on the side
          fieldA String    @unique // comment on the side
          // comment before
          /// doc comment before
          anotherWeirdFieldName Int
        }
    "#};

    let expected = expect![[r#"
        /// model doc comment
        model User {
          id                    Int    @id // doc comment on the side
          fieldA                String @unique // comment on the side
          // comment before
          /// doc comment before
          anotherWeirdFieldName Int
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn catch_all_in_a_block_must_not_influence_table_layout() {
    let input = indoc! {r#"
        model Post {
          id   Int @id
          this is an invalid line
          anotherField String
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id           Int    @id
          this is an invalid line
          anotherField String
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn format_should_enforce_order_of_field_attributes() {
    let input = indoc! {r#"
        model Post {
          id        Int      @default(autoincrement()) @id
          published Boolean  @map("_published") @default(false)
          author    User?   @relation(fields: [authorId], references: [id])
          authorId  Int?
        }

        model User {
          megaField DateTime @map("mega_field") @id @default("_megaField") @unique @updatedAt
        }

        model Test {
          id     Int   @id @map("_id") @default(1) @updatedAt
          blogId Int?  @unique @default(1)
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id        Int     @id @default(autoincrement())
          published Boolean @default(false) @map("_published")
          author    User?   @relation(fields: [authorId], references: [id])
          authorId  Int?
        }

        model User {
          megaField DateTime @id @unique @default("_megaField") @updatedAt @map("mega_field")
        }

        model Test {
          id     Int  @id @default(1) @updatedAt @map("_id")
          blogId Int? @unique @default(1)
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn format_should_enforce_order_of_block_attributes() {
    let input = indoc! {r#"
        model Person {
          firstName   String
          lastName    String
          codeName    String
          yearOfBirth Int
          @@map("blog")
          @@index([yearOfBirth])
          @@unique([codeName, yearOfBirth])
          @@id([firstName, lastName])
        }

        model Blog {
          id    Int    @default(1)
          name  String
          posts Post[]
          @@id([id])
          @@index([id, name])
          @@unique([name])
          @@map("blog")
        }
    "#};

    let expected = expect![[r#"
        model Person {
          firstName   String
          lastName    String
          codeName    String
          yearOfBirth Int

          @@id([firstName, lastName])
          @@unique([codeName, yearOfBirth])
          @@index([yearOfBirth])
          @@map("blog")
        }

        model Blog {
          id    Int    @default(1)
          name  String
          posts Post[]

          @@id([id])
          @@unique([name])
          @@index([id, name])
          @@map("blog")
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
#[ignore]
fn format_should_put_block_attributes_to_end_of_block_with_comments() {
    let input = indoc! {r#"
        model Blog {
          @@id([id1, id2]) /// id comment
          id1 Int
          id2 Int
          @@map("blog") /// blog comment
        }
    "#};

    let expected = expect![[r#"
        model Blog {
          id1 Int
          id2 Int

          @@map("blog") /// blog comment
          @@id([id1, id2]) /// id comment
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn format_should_put_block_attributes_to_end_of_block_without_comments() {
    let input = indoc! {r#"
        model Blog {
          @@map("blog")
          id1 Int
          id2 Int
          @@id([id1, id2])
        }
    "#};

    let expected = expect![[r#"
        model Blog {
          id1 Int
          id2 Int

          @@id([id1, id2])
          @@map("blog")
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn comments_in_a_model_must_not_move() {
    let input = indoc! {r#"
        model User {
          id     Int    @id
          // Comment
          email  String @unique
          // Comment 2
        }
    "#};

    let expected = expect![[r#"
        model User {
          id    Int    @id
          // Comment
          email String @unique
          // Comment 2
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn end_of_line_comments_must_not_influence_table_layout_in_models() {
    let input = indoc! {r#"
        model Test {
          id  Int   @id    // Comment 1
          foo String     // Comment 2
          bar bar? @relation(fields: [id], references: [id]) // Comment 3
        }
    "#};

    let expected = expect![[r#"
        model Test {
          id  Int    @id // Comment 1
          foo String // Comment 2
          bar bar?   @relation(fields: [id], references: [id]) // Comment 3
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn end_of_line_comments_must_not_influence_table_layout_in_enums() {
    let input = indoc! {r#"
        enum Foo {
          ONE @map("short")     // COMMENT 1
          TWO @map("a_very_long_name")    // COMMENT 2
        }
    "#};

    let expected = expect![[r#"
        enum Foo {
          ONE  @map("short") // COMMENT 1
          TWO  @map("a_very_long_name") // COMMENT 2
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn commented_models_dont_get_removed() {
    let input = indoc! {r#"
        // model One {
        //   id Int @id
        // }

        model Two {
          id Int @id
        }

        // model Three {
        //   id Int @id
        // }
    "#};

    let expected = expect![[r#"
        // model One {
        //   id Int @id
        // }

        model Two {
          id Int @id
        }

        // model Three {
        //   id Int @id
        // }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn a_comment_in_datasource_must_not_add_extra_newlines() {
    let input = indoc! {r#"
        datasource pg {
          provider = "postgresql"
          url = "postgresql://"
          // a comment
        }
    "#};

    let expected = expect![[r#"
        datasource pg {
          provider = "postgresql"
          url      = "postgresql://"
          // a comment
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn a_comment_in_generator_must_not_add_extra_newlines() {
    let input = indoc! {r#"
        generator js {
            provider = "js"
            // a comment
        }
    "#};

    let expected = expect![[r#"
        generator js {
          provider = "js"
          // a comment
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn test_reformat_config() {
    let input = indoc! {r#"
        datasource pg {
          provider = "postgresql"
          url = "postgresql://"
        }
    "#};

    let expected = expect![[r#"
        datasource pg {
          provider = "postgresql"
          url      = "postgresql://"
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn test_reformat_tabs() {
    let input = indoc! {r#"
        datasource pg {
          provider\t=\t"postgresql"
          url = "postgresql://"
        }
    "#};

    let expected = expect![[r#"
        datasource pg {
          provider = "postgresql"
          url      = "postgresql://"
        }
    "#]];

    let result = Reformatter::new(&input.replace("\\t", "\t")).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn test_floating_doc_comments_1() {
    let input = indoc! {r#"
        model a {
          one Int
          two Int
          // bs  b[] @relation(references: [a])
          @@id([one, two])
        }

        /// ajlsdkfkjasflk
        // model ok {}
    "#};

    let expected = expect![[r#"
        model a {
          one Int
          two Int
          // bs  b[] @relation(references: [a])

          @@id([one, two])
        }

        /// ajlsdkfkjasflk
        // model ok {}
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn test_floating_doc_comments_2() {
    let input = indoc! {r#"
        model a {
          one Int
          two Int
          // bs  b[] @relation(references: [a])

          @@id([one, two])
        }

        // ajlsdkfkjasflk
        // ajlsdkfkjasflk
    "#};

    let expected = expect![[r#"
        model a {
          one Int
          two Int
          // bs  b[] @relation(references: [a])

          @@id([one, two])
        }

        // ajlsdkfkjasflk
        // ajlsdkfkjasflk
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn reformatting_enums_must_work() {
    let input = indoc! {r#"
        enum Colors {
          RED @map("rett")
          BLUE
          GREEN

          // comment
          ORANGE_AND_KIND_OF_RED @map("super_color")

          @@map("the_colors")
        }
  "#};

    let expected = expect![[r#"
        enum Colors {
          RED    @map("rett")
          BLUE
          GREEN

          // comment
          ORANGE_AND_KIND_OF_RED  @map("super_color")

          @@map("the_colors")
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn reformatting_must_work_when_env_var_is_missing() {
    let input = indoc! {r#"
        datasource pg {
          provider = "postgresql"
          url = env("DATABASE_URL")
        }
    "#};

    let expected = expect![[r#"
        datasource pg {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn invalid_lines_must_not_break_reformatting() {
    let input = indoc! {r#"
        $ /a/b/c:.
        model Post {
          id Int @id
        }
    "#};

    let expected = expect![[r#"
        $ /a/b/c:.
        model Post {
          id Int @id
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn reformatting_an_invalid_datasource_block_must_work() {
    let input = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url = env("POSTGRESQL_URL")
          test
        }
    "#};

    let expected = expect![[r#"
        datasource db {
          provider = "postgresql"
          url      = env("POSTGRESQL_URL")
          test
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn reformatting_an_invalid_generator_block_must_work() {
    let input = indoc! {r#"
        generator js {
          provider = "js"
          output = "../wherever"
          test
        }
    "#};

    let expected = expect![[r#"
        generator js {
          provider = "js"
          output   = "../wherever"
          test
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn reformatting_a_model_with_native_type_definitions_must_work() {
    let input = indoc! {r#"
        datasource pg {
          provider = "postgres"
          url      = "postgresql://"
        }

        model Blog {
          id     Int    @id
          bigInt Int    @pg.Integer
          foobar String @pg.VarChar(12)
        }
    "#};

    let expected = expect![[r#"
        datasource pg {
          provider = "postgres"
          url      = "postgresql://"
        }

        model Blog {
          id     Int    @id
          bigInt Int    @pg.Integer
          foobar String @pg.VarChar(12)
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn incomplete_field_definitions_in_a_model_must_not_get_removed() {
    // incomplete field definitions are handled in a special way in the grammar to allow nice errors. See `nice_error.rs:nice_error_missing_type`
    // Hence the block level catch does not apply here. So we must test this specifically.
    let input = indoc! {r#"
        model Post {
          id   Int      @id
          tags String[]
          test // an incomplete field
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id   Int      @id
          tags String[]
          test // an incomplete field
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn new_lines_inside_block_above_field_must_stay() {
    let input = indoc! {r#"
        model Post {




          id Int @id @default(autoincrement())
        }
    "#};

    let expected = expect![[r#"
        model Post {




          id Int @id @default(autoincrement())
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn new_lines_inside_block_below_field_must_stay() {
    let input = indoc! {r#"
        model Post {
          id Int @id @default(autoincrement())




        }
    "#};

    let expected = expect![[r#"
        model Post {
          id Int @id @default(autoincrement())




        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn new_lines_inside_block_in_between_fields_must_stay() {
    let input = indoc! {r#"
        model Post {
          id Int @id @default(autoincrement())


          input String

        }
    "#};

    let expected = expect![[r#"
        model Post {
          id Int @id @default(autoincrement())


          input String

        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn new_lines_before_first_block_must_be_removed() {
    let input = indoc! {r#"

        model Post {
          id Int @id
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id Int @id
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn new_lines_between_blocks_must_be_reduced_to_one_simple() {
    let input = indoc! {r#"
        model Post {
          id Int @id
        }


        model Blog {
          id Int @id
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id Int @id
        }

        model Blog {
          id Int @id
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn multiple_new_lines_between_top_level_elements_must_be_reduced_to_a_single_one() {
    let input = indoc! {r#"
        model Post {
          id Int @id
        }


        // free floating comment
        /// free floating doc comment


        // model comment
        /// model doc comment
        model Blog {
          id Int @id
        }


        // free floating comment
        /// free floating doc comment


        /// source doc comment
        // source comment
        datasource mydb {
          provider = "sqlite"
          url      = "file:dev.db"
        }


        // free floating comment
        /// free floating doc comment

        // enum comment
        /// enum doc comment
        enum Status {
          ACTIVE
          DONE
        }


        // free floating comment
        /// free floating doc comment

        // type alias comment
        /// type alias doc comment
        type MyString = String          @default("FooBar")


        // free floating comment
        /// free floating doc comment

        // generator comment
        /// generator doc comment
        generator js {
            provider = "js"
        }


        // free floating comment
        /// free floating doc comment

        /// another model doc comment
        // another model comment
        model Comment {
          id Int @id
        }
    "#};

    // TODO: the formatting of the type alias is not nice
    let expected = expect![[r#"
        model Post {
          id Int @id
        }

        // free floating comment
        /// free floating doc comment

        // model comment
        /// model doc comment
        model Blog {
          id Int @id
        }

        // free floating comment
        /// free floating doc comment

        /// source doc comment
        // source comment
        datasource mydb {
          provider = "sqlite"
          url      = "file:dev.db"
        }

        // free floating comment
        /// free floating doc comment

        // enum comment
        /// enum doc comment
        enum Status {
          ACTIVE
          DONE
        }

        // free floating comment
        /// free floating doc comment

        // type alias comment
        /// type alias doc comment
        type                       MyString = String @default("FooBar")

        // free floating comment
        /// free floating doc comment

        // generator comment
        /// generator doc comment
        generator js {
          provider = "js"
        }

        // free floating comment
        /// free floating doc comment

        /// another model doc comment
        // another model comment
        model Comment {
          id Int @id
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn model_level_attributes_reset_the_table_layout() {
    let input = indoc! {r#"
        model Post {
          id Int @id
          aVeryLongName  String
          alsoAVeryLongName String

          @@index([a])
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id                Int    @id
          aVeryLongName     String
          alsoAVeryLongName String

          @@index([a])
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn incomplete_last_line_must_not_stop_reformatting() {
    // https://github.com/prisma/vscode/issues/140
    // If a user types on the very last line we did not error nicely.
    // a new line fixed the problem but this is not nice.
    let input = indoc! {r#"
        model User {
          id       Int       @id
        }
        model Bl
    "#};

    let expected = expect![[r#"
        model User {
          id Int @id
        }
        model Bl
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn unsupported_is_allowed() {
    let input = indoc! {r#"
        model Post {
          id Int @id
          required Unsupported("some type")
          optional Unsupported("some type")?
          list Unsupported("some type")[]
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id       Int                        @id
          required Unsupported("some type")
          optional Unsupported("some type")?
          list     Unsupported("some type")[]
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn ignore_is_allowed() {
    let input = indoc! {r#"
        model Post {
          id Int @id
          @@ignore
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id Int @id

          @@ignore
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn db_generated_is_allowed() {
    let input = indoc! {r#"
        model Post {
          id Int @id              @default(dbgenerated("something"))
        }
    "#};

    let expected = expect![[r#"
        model Post {
          id Int @id @default(dbgenerated("something"))
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn reformatting_ignore_with_relations_works() {
    let input = indoc! {r#"
        model client {
          client_id                 Int                         @id
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model order {
          client_id                  Int?
          client                     client?  @relation(fields: [client_id], references: [client_id])

          @@ignore
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model bill {
          client_id                  Int?
          client                     client?  @relation(fields: [client_id], references: [client_id])

          @@ignore
        }
    "#};

    let expected = expect![[r#"
        model client {
          client_id Int     @id
          order     order[] @ignore
          bill      bill[]  @ignore
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model order {
          client_id Int?
          client    client? @relation(fields: [client_id], references: [client_id])

          @@ignore
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model bill {
          client_id Int?
          client    client? @relation(fields: [client_id], references: [client_id])

          @@ignore
        }
    "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}

#[test]
fn composite_types_are_not_reformatted_into_models() {
    let input = indoc! {r#"
      type User {
        id       Int       @id
      }
    "#};

    let expected = expect![[r#"
      type User {
        id Int @id
      }
  "#]];

    let result = Reformatter::new(input).reformat_to_string();
    expected.assert_eq(&result);
}
