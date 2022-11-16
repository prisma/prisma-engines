use crate::common::*;

fn reformat(input: &str) -> String {
    psl::reformat(input, 2).unwrap_or_else(|| input.to_owned())
}

#[test]
fn must_add_new_line_to_end_of_schema() {
    let input = r#"// a comment"#;

    let expected = expect![[r#"
        // a comment
    "#]];

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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
          ONE @map("short") // COMMENT 1
          TWO @map("a_very_long_name") // COMMENT 2
        }
    "#]];

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(&input.replace("\\t", "\t")));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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
          RED   @map("rett")
          BLUE
          GREEN

          // comment
          ORANGE_AND_KIND_OF_RED @map("super_color")

          @@map("the_colors")
        }
    "#]];

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
}

#[test]
fn new_lines_inside_block_above_field_must_go_away() {
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

    expected.assert_eq(&reformat(input));
}

#[test]
fn new_lines_inside_block_below_field_must_go_away() {
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

    expected.assert_eq(&reformat(input));
}

#[test]
fn new_lines_inside_block_in_between_fields_must_go_away() {
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
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

    expected.assert_eq(&reformat(input));
}

#[test]
fn reformatting_extended_indexes_works() {
    let input = indoc! {r#"
        generator client {
          provider        = "prisma-client-js"
          binaryTargets   = ["darwin"]
        }
        
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }
        
        model A {
          id   Int    @id
          name String @unique(length: 15, sort: Desc)
          a    String
          b    String
          B    B[]    @relation("AtoB")
        
          @@unique([a, b], map: "compound")
          @@index([a(sort: Desc, length: 100)], map: "A_a_idx")
        }
        
        model B {
          a   String
          b   String
          aId Int
          A   A      @relation("AtoB", fields: [aId], references: [id])
        
          @@id([a, b])
          @@index([aId], map: "B_aId_idx")
        }
    "#};

    let expected = expect![[r#"
        generator client {
          provider      = "prisma-client-js"
          binaryTargets = ["darwin"]
        }

        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model A {
          id   Int    @id
          name String @unique(length: 15, sort: Desc)
          a    String
          b    String
          B    B[]    @relation("AtoB")

          @@unique([a, b], map: "compound")
          @@index([a(sort: Desc, length: 100)], map: "A_a_idx")
        }

        model B {
          a   String
          b   String
          aId Int
          A   A      @relation("AtoB", fields: [aId], references: [id])

          @@id([a, b])
          @@index([aId], map: "B_aId_idx")
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn reformatting_with_empty_indexes() {
    let schema = r#"
        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["fullTextIndex"]
        }

        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model Fulltext {
          id      Int    @id
          title   String @db.VarChar(255)
          content String @db.Text

          @@fulltext(fields:[], map: "a")
          @@index(fields: [ ], map: "b")
          @@unique(fields: [])
        }
    "#;

    let expected = expect![[r#"
        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["fullTextIndex"]
        }

        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model Fulltext {
          id      Int    @id
          title   String @db.VarChar(255)
          content String @db.Text

          @@unique(fields: [])
          @@index(fields: [], map: "b")
          @@fulltext(fields: [], map: "a")
        }
    "#]];

    expected.assert_eq(&reformat(schema));
}

#[test]
fn test_composite_types_in_models() {
    let input = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url      = "mongodb://prisma:prisma@127.0.0.1:27017/test?authSource=admin"
        }

        generator js {
          previewFeatures = ["mongodb"]
          provider        = "prisma-client-js"
        }

        model A {
          id String @id @default(auto()) @map("_id") @db.ObjectId
          b  B
          c  C[]
        }

        type B {
          b_1 String
          b_2 Int
        }

        type C {
          b_1 String
          b_2 Int
        }
    "#};

    let expected = expect![[r#"
        datasource db {
          provider = "mongodb"
          url      = "mongodb://prisma:prisma@127.0.0.1:27017/test?authSource=admin"
        }

        generator js {
          previewFeatures = ["mongodb"]
          provider        = "prisma-client-js"
        }

        model A {
          id String @id @default(auto()) @map("_id") @db.ObjectId
          b  B
          c  C[]
        }

        type B {
          b_1 String
          b_2 Int
        }

        type C {
          b_1 String
          b_2 Int
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn empty_arguments_reformat_properly() {
    let schema = r#"
        /// Post including an author and content.
        model Post {
          id        Int     @id @default(autoincrement())
          content   String? @default(map: "")
          published Boolean @default(false)
          author    User?   @relation(fields: [authorId], references: [id], onDelete: )
          authorId  Int?
        }

        // Documentation for this model.
        model User {
          id    Int     @id @default(autoincrement())
          email String  @unique
          name  String?
          posts Post[]
        }
    "#;

    let expected = expect![[r#"
        /// Post including an author and content.
        model Post {
          id        Int     @id @default(autoincrement())
          content   String? @default(map: "")
          published Boolean @default(false)
          author    User?   @relation(fields: [authorId], references: [id], onDelete: )
          authorId  Int?
        }

        // Documentation for this model.
        model User {
          id    Int     @id @default(autoincrement())
          email String  @unique
          name  String?
          posts Post[]
        }
    "#]];

    expected.assert_eq(&reformat(schema));
}

#[test]
fn composite_type_native_types_roundtrip() {
    let schema = r#"
        datasource db{
            provider = "mongodb"
            url = "mongo+srv:/...."
        }

        generator client {
            provider        = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        type Address {
            name String?
            street String @db.ObjectId
            number Int
            zipCode Int?
        }

        model User {
            id  String @id @default(dbgenerated()) @map("_id") @db.ObjectId
            address Address?
        }
    "#;

    let expected = expect![[r#"
        datasource db {
          provider = "mongodb"
          url      = "mongo+srv:/...."
        }

        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["mongoDb"]
        }

        type Address {
          name    String?
          street  String  @db.ObjectId
          number  Int
          zipCode Int?
        }

        model User {
          id      String   @id @default(dbgenerated()) @map("_id") @db.ObjectId
          address Address?
        }
    "#]];

    expected.assert_eq(&reformat(schema));
}

#[test]
fn removes_legacy_colon_from_fields() {
    let input = indoc! {r#"
        model Site {
          name: String
          htmlTitle: String
        }
    "#};

    let expected = expect![[r#"
        model Site {
          name      String
          htmlTitle String
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn rewrites_legacy_list_and_required_type_arities() {
    let input = indoc! {r#"
        model Site {
          name String!
          htmlTitles [String]
        }
    "#};

    let expected = expect![[r#"
        model Site {
          name       String
          htmlTitles String[]
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn attribute_arguments_reformatting_is_idempotent() {
    let schema = r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = "mongodb"
        }

        datasource db {
          provider = "mongodb"
          url      = "m...ty"
        }

        model Foo {
          id       String   @id @default(auto()) @map("_id") @db.ObjectId
          name     String   @unique
          json     Json
          bar      Bar
          bars     Bar[]
          baz      Baz      @relation(fields: [bazId], references: [id])
          bazId    String   @db.ObjectId
          list     String[]
          jsonList Json[]
        }

        type Bar {
          label  String
          number Int
        }

        model Baz {
          id  String @id @default(auto()) @map("_id") @db.ObjectId
          foo Foo?
        }
    "#;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = "mongodb"
        }

        datasource db {
          provider = "mongodb"
          url      = "m...ty"
        }

        model Foo {
          id       String   @id @default(auto()) @map("_id") @db.ObjectId
          name     String   @unique
          json     Json
          bar      Bar
          bars     Bar[]
          baz      Baz      @relation(fields: [bazId], references: [id])
          bazId    String   @db.ObjectId
          list     String[]
          jsonList Json[]
        }

        type Bar {
          label  String
          number Int
        }

        model Baz {
          id  String @id @default(auto()) @map("_id") @db.ObjectId
          foo Foo?
        }
    "#]];
    let reformatted = reformat(schema);
    expected.assert_eq(&reformatted);
    assert_eq!(reformatted, reformat(&reformatted)); // it's idempotent
}
