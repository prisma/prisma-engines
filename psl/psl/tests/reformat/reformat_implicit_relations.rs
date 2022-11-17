use crate::common::*;

#[test]
fn back_relation_fields_missing_attributes_should_not_add_attributes_multiple_times() {
    let input = indoc! {r#"
        model User {
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
    "#};

    let expected = expect![[r#"
        model User {
          id     Int  @id
          post   Post @relation(fields: [postId], references: [id])
          postId Int
        }

        model Post {
          id   Int    @id
          User User[]
          Cat  Cat[]
        }

        model Cat {
          id     Int  @id
          post   Post @relation(fields: [postId], references: [id])
          postId Int
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn back_relations_must_be_added_when_attribute_is_present_with_no_arguments() {
    let input = indoc! {r#"
        model User {
          id Int @id
          post Post @relation
        }

        model Post {
          id Int @id
        }
    "#};

    let expected = expect![[r#"
        model User {
          id     Int  @id
          post   Post @relation(fields: [postId], references: [id])
          postId Int
        }

        model Post {
          id   Int    @id
          User User[]
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn scalar_field_and_attribute_must_be_added_even_when_attribute_is_missing_and_both_relation_fields_present() {
    let input = indoc! {r#"
        model User {
          id Int @id
          post Post
        }

        model Post {
          id Int @id
          User User[]
        }
        "#
    };

    let expected = expect![[r#"
        model User {
          id     Int  @id
          post   Post @relation(fields: [postId], references: [id])
          postId Int
        }

        model Post {
          id   Int    @id
          User User[]
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn scalar_field_and_attribute_must_be_added_even_when_attribute_is_missing_and_only_one_relation_field_present() {
    let input = indoc! {r#"
        model User {
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
        "#
    };

    let expected = expect![[r#"
        model User {
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
          postId Int
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn back_relations_must_be_added_even_when_attribute_is_missing_for_one_to_one() {
    let input = indoc! {r#"
        model User {
          id     Int   @id
          Post   Post?
        }

        model Post {
          id   Int    @id
          User User
        }
        "#
    };

    let expected = expect![[r#"
        model User {
          id   Int   @id
          Post Post?
        }

        model Post {
          id     Int  @id
          User   User @relation(fields: [userId], references: [id])
          userId Int
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn back_relations_and_attribute_must_be_added_even_when_attribute_is_missing_for_one_to_many() {
    let input = indoc! {r#"
        model User {
          id     Int   @id
          Post   Post
        }

        model Post {
          id   Int    @id
          User User[]
        }
        "#
    };

    let expected = expect![[r#"
        model User {
          id     Int  @id
          Post   Post @relation(fields: [postId], references: [id])
          postId Int
        }

        model Post {
          id   Int    @id
          User User[]
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn relation_attribute_must_not_be_added_for_many_to_many() {
    let input = indoc! {r#"
        model User {
          id   Int    @id
          Post Post[]
        }

        model Post {
          id   Int    @id
          User User[]
        }
        "#
    };

    let expected = expect![[r#"
        model User {
          id   Int    @id
          Post Post[]
        }

        model Post {
          id   Int    @id
          User User[]
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn must_add_relation_attribute_to_an_existing_field() {
    let input = indoc! {r#"
        model Blog {
          id    Int     @id
          posts Post[]
        }

        model Post {
          id     Int   @id
          Blog   Blog? @relation(fields: [blogId])
          blogId Int?
        }
        "#
    };

    let expected = expect![[r#"
        model Blog {
          id    Int    @id
          posts Post[]
        }

        model Post {
          id     Int   @id
          Blog   Blog? @relation(fields: [blogId], references: [id])
          blogId Int?
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn must_add_back_relation_fields_for_given_list_field() {
    let input = indoc! {r#"
        model User {
          id Int @id
          posts Post[]
        }

        model Post {
          post_id Int @id
        }
    "#};

    let expected = expect![[r#"
        model User {
          id    Int    @id
          posts Post[]
        }

        model Post {
          post_id Int   @id
          User    User? @relation(fields: [userId], references: [id])
          userId  Int?
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn must_add_back_relation_fields_for_given_singular_field() {
    let input = indoc! {r#"
        model User {
          id     Int @id
          postId Int
          post   Post @relation(fields: [postId], references: [post_id])
        }

        model Post {
          post_id Int @id
        }
    "#};

    let expected = expect![[r#"
        model User {
          id     Int  @id
          postId Int
          post   Post @relation(fields: [postId], references: [post_id])
        }

        model Post {
          post_id Int    @id
          User    User[]
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn must_add_back_relation_fields_for_self_relations() {
    let input = indoc! {r#"
        model Human {
          id    Int @id
          sonId Int?
          son   Human? @relation(fields: [sonId], references: [id])
        }
    "#};

    let expected = expect![[r#"
        model Human {
          id    Int     @id
          sonId Int?
          son   Human?  @relation(fields: [sonId], references: [id])
          Human Human[]
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn should_camel_case_back_relation_field_name() {
    let input = indoc! {r#"
        model OhWhatAUser {
          id Int @id
          posts Post[]
        }

        model Post {
          post_id Int @id
        }
    "#};

    let expected = expect![[r#"
        model OhWhatAUser {
          id    Int    @id
          posts Post[]
        }

        model Post {
          post_id       Int          @id
          OhWhatAUser   OhWhatAUser? @relation(fields: [ohWhatAUserId], references: [id])
          ohWhatAUserId Int?
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
//todo I dont like that mother and User field are both the same relation but only one side prints its relationname
fn add_backrelation_for_unambiguous_self_relations_in_presence_of_unrelated_other_relations() {
    let input = indoc! {r#"
        model User {
          id          Int @id
          motherId    Int
          mother      User @relation(fields: motherId, references: id)
          subscribers Follower[]
        }

        model Follower {
          id        Int   @id
          following User[]
        }
    "#};

    let expected = expect![[r#"
        model User {
          id          Int        @id
          motherId    Int
          mother      User       @relation(fields: motherId, references: id)
          subscribers Follower[]
          User        User[]
        }

        model Follower {
          id        Int    @id
          following User[]
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn must_succeed_when_fields_argument_is_missing_for_one_to_many() {
    let input = indoc! {r#"
        model User {
          id        Int @id
          firstName String
          posts     Post[]
        }

        model Post {
          id     Int     @id
          userId Int
          user   User    @relation(references: [id])
        }
    "#};

    let expected = expect![[r#"
        model User {
          id        Int    @id
          firstName String
          posts     Post[]
        }

        model Post {
          id     Int  @id
          userId Int
          user   User @relation(references: [id], fields: [userId])
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn must_add_referenced_fields_for_one_to_many_relations() {
    let input = indoc! {r#"
        model User {
          user_id Int    @id
          posts   Post[]
        }

        model Post {
          post_id Int    @id
          user    User
        }
    "#};

    let expected = expect![[r#"
        model User {
          user_id Int    @id
          posts   Post[]
        }

        model Post {
          post_id     Int  @id
          user        User @relation(fields: [userUser_id], references: [user_id])
          userUser_id Int
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn must_add_referenced_fields_for_one_to_many_relations_reverse() {
    let input = indoc! {r#"
        model User {
          user_id Int    @id
          post    Post
        }

        model Post {
          post_id Int    @id
          users   User[]
        }
    "#};

    let expected = expect![[r#"
        model User {
          user_id     Int  @id
          post        Post @relation(fields: [postPost_id], references: [post_id])
          postPost_id Int
        }

        model Post {
          post_id Int    @id
          users   User[]
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn must_add_referenced_fields_on_the_right_side_for_one_to_one_relations() {
    // the to fields are always added to model with the lower name in lexicographic order

    let input = indoc! {r#"
        model User1 {
          id         String @id @default(cuid())
          referenceA User2?
        }

        model User2 {
          id         String @id @default(cuid())
          referenceB User1?
        }

        model User3 {
          id         String @id @default(cuid())
          referenceB User4?
        }

        model User4 {
          id         String @id @default(cuid())
          referenceA User3?
        }
    "#};

    let expected = expect![[r#"
        model User1 {
          id         String  @id @default(cuid())
          referenceA User2?  @relation(fields: [user2Id], references: [id])
          user2Id    String?
        }

        model User2 {
          id         String @id @default(cuid())
          referenceB User1?
        }

        model User3 {
          id         String  @id @default(cuid())
          referenceB User4?  @relation(fields: [user4Id], references: [id])
          user4Id    String?
        }

        model User4 {
          id         String @id @default(cuid())
          referenceA User3?
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn must_handle_conflicts_with_existing_fields_if_types_are_compatible() {
    let input = indoc! {r#"
        model Blog {
          id    String @id
          posts Post[]
        }

        model Post {
          id     String   @id
          blogId String?
        }
    "#};

    let expected = expect![[r#"
        model Blog {
          id    String @id
          posts Post[]
        }

        model Post {
          id     String  @id
          blogId String?
          Blog   Blog?   @relation(fields: [blogId], references: [id])
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn forward_relation_field_generation_picks_up_types_of_existing_underlying_scalar_fields() {
    let input = indoc! {r#"
        model Blog {
          id    String @id
          posts Post[]
        }

        model Post {
          id     String   @id
          blogId Int?     // this is not compatible with Blog.id
        }
    "#};

    let expected = expect![[r#"
        model Blog {
          id    String @id
          posts Post[]
        }

        model Post {
          id     String @id
          blogId Int? // this is not compatible with Blog.id
          Blog   Blog?  @relation(fields: [blogId], references: [id])
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn should_add_back_relations_for_more_complex_cases() {
    let input = indoc! {r#"
        model User {
          id Int @id
          posts Post[]
        }

        model Post {
          post_id Int @id
          comments Comment[]
          categories PostToCategory[]
        }

        model Comment {
          comment_id Int @id
        }

        model Category {
          category_id Int @id
          posts PostToCategory[]
        }

        model PostToCategory {
          id          Int @id
          postId      Int
          categoryId  Int

          post     Post     @relation(fields: [postId], references: [post_id])
          category Category @relation(fields: [categoryId], references: [category_id])
          @@map("post_to_category")
        }
    "#};

    let expected = expect![[r#"
        model User {
          id    Int    @id
          posts Post[]
        }

        model Post {
          post_id    Int              @id
          comments   Comment[]
          categories PostToCategory[]
          User       User?            @relation(fields: [userId], references: [id])
          userId     Int?
        }

        model Comment {
          comment_id  Int   @id
          Post        Post? @relation(fields: [postPost_id], references: [post_id])
          postPost_id Int?
        }

        model Category {
          category_id Int              @id
          posts       PostToCategory[]
        }

        model PostToCategory {
          id         Int @id
          postId     Int
          categoryId Int

          post     Post     @relation(fields: [postId], references: [post_id])
          category Category @relation(fields: [categoryId], references: [category_id])

          @@map("post_to_category")
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn should_add_missing_embed_ids_on_self_relations() {
    let input = indoc! {r#"
        model Human {
          id Int @id
          father Human? @relation("paternity")
          son Human? @relation("paternity")
        }
    "#};

    let expected = expect![[r#"
        model Human {
          id      Int    @id
          father  Human? @relation("paternity", fields: [humanId], references: [id])
          son     Human? @relation("paternity")
          humanId Int?
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn should_add_referenced_fields_on_the_correct_side_list() {
    let input = indoc! {r#"
        model User {
          id Int @id
          post Post[]
        }

        model Post {
          post_id Int @id
          user User
        }
    "#};

    let expected = expect![[r#"
        model User {
          id   Int    @id
          post Post[]
        }

        model Post {
          post_id Int  @id
          user    User @relation(fields: [userId], references: [id])
          userId  Int
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn no_changes_for_many_to_many_relations() {
    let input = indoc! {r#"
        model User {
          user_id Int    @id
          posts   Post[]
        }

        model Post {
          post_id Int    @id
          users   User[]
        }
    "#};

    let expected = expect![[r#"
        model User {
          user_id Int    @id
          posts   Post[]
        }

        model Post {
          post_id Int    @id
          users   User[]
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn should_add_referenced_fields_on_the_correct_side_tie_breaker() {
    let input = indoc! {r#"
        model User {
          user_id Int @id
          post Post?
        }

        model Post {
          post_id Int @id
          user User?
        }
    "#};

    let expected = expect![[r#"
        model User {
          user_id Int   @id
          post    Post?
        }

        model Post {
          post_id     Int   @id
          user        User? @relation(fields: [userUser_id], references: [user_id])
          userUser_id Int?
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn should_not_get_confused_with_complicated_self_relations() {
    let input = indoc! {r#"
        model Human {
          id        Int  @id
          husbandId Int? @unique
          fatherId  Int? @unique
          parentId  Int?

          wife    Human? @relation("Marrige")
          husband Human? @relation("Marrige", fields: husbandId, references: id)

          father Human? @relation("Paternity", fields: fatherId, references: id)
          son    Human? @relation("Paternity")

          children Human[] @relation("Offspring")
          parent   Human?  @relation("Offspring", fields: parentId, references: id)
        }
    "#};

    let expected = expect![[r#"
        model Human {
          id        Int  @id
          husbandId Int? @unique
          fatherId  Int? @unique
          parentId  Int?

          wife    Human? @relation("Marrige")
          husband Human? @relation("Marrige", fields: husbandId, references: id)

          father Human? @relation("Paternity", fields: fatherId, references: id)
          son    Human? @relation("Paternity")

          children Human[] @relation("Offspring")
          parent   Human?  @relation("Offspring", fields: parentId, references: id)
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn back_relations_must_be_added_even_when_env_vars_are_missing() {
    let input = indoc! {r#"
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
    "#};

    let expected = expect![[r#"
        datasource db {
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
    "#]];

    expected.assert_eq(&reformat(input));
}

#[test]
fn must_add_required_relation_field_if_underlying_scalar_is_required() {
    let input = indoc! {r#"
        model Session {
          id       Int @id
          userId   Int
        }

        model User {
          id       Int       @id
          sessions Session[]
        }

        model Session2 {
          id         Int @id
          user2Id    Int
          user2Id2   Int
        }

        model User2 {
          id       Int
          id2      Int
          sessions Session2[]

          @@id([id, id2])
        }

        model Session3 {
          id         Int @id
          user3Id    Int?
          user3Id2   Int
        }

        model User3 {
          id       Int
          id2      Int
          sessions Session3[]

          @@id([id, id2])
        }
    "#};

    let expected = expect![[r#"
        model Session {
          id     Int  @id
          userId Int
          User   User @relation(fields: [userId], references: [id])
        }

        model User {
          id       Int       @id
          sessions Session[]
        }

        model Session2 {
          id       Int   @id
          user2Id  Int
          user2Id2 Int
          User2    User2 @relation(fields: [user2Id, user2Id2], references: [id, id2])
        }

        model User2 {
          id       Int
          id2      Int
          sessions Session2[]

          @@id([id, id2])
        }

        model Session3 {
          id       Int    @id
          user3Id  Int?
          user3Id2 Int
          User3    User3? @relation(fields: [user3Id, user3Id2], references: [id, id2])
        }

        model User3 {
          id       Int
          id2      Int
          sessions Session3[]

          @@id([id, id2])
        }
    "#]];

    expected.assert_eq(&reformat(input));
}

// this crashed
#[test]
fn issue_10118() {
    let schema = r#"
        datasource db {
          provider = "postgres"
          url = "postgres://"
        }

        model User {
          id         String    @id @default(cuid()) @db.Char(30)
          referralId String    @unique @db.Char(30)
          referral   Referral? @relation("UserToReferral", fields: [referralId], references: [id])
        }

        model Referral {
          id   String @id @default(cuid()) @db.Char(30)
          user User   @relation("UserToReferral")
        }
    "#;

    let expected = expect![[r#"
        datasource db {
          provider = "postgres"
          url      = "postgres://"
        }

        model User {
          id         String    @id @default(cuid()) @db.Char(30)
          referralId String    @unique @db.Char(30)
          referral   Referral? @relation("UserToReferral", fields: [referralId], references: [id])
        }

        model Referral {
          id     String @id @default(cuid()) @db.Char(30)
          user   User   @relation("UserToReferral", fields: [userId], references: [id])
          userId String @db.Char(30)
        }
    "#]];
    expected.assert_eq(&reformat(schema));
}

#[test]
fn mongodb_inline_relations_reformat_as_expected() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        generator js {
          provider = "prisma-client-js"
          previewFeatures = ["mongoDb"]
        }

        model A {
          id   String   @id @map("_id") @default(auto()) @db.ObjectId
          bIds String[] @db.ObjectId
          bs   B[]      @relation(fields: [bIds])
        }

        model B {
          id   String   @id @map("_id") @default(auto()) @db.ObjectId
        }
    "#};

    let expected = expect![[r#"
        datasource db {
          provider = "mongodb"
          url      = "mongodb://"
        }

        generator js {
          provider        = "prisma-client-js"
          previewFeatures = ["mongoDb"]
        }

        model A {
          id   String   @id @default(auto()) @map("_id") @db.ObjectId
          bIds String[] @db.ObjectId
          bs   B[]      @relation(fields: [bIds])
        }

        model B {
          id  String  @id @default(auto()) @map("_id") @db.ObjectId
          A   A?      @relation(fields: [aId], references: [id])
          aId String? @db.ObjectId
        }
    "#]];

    expected.assert_eq(&reformat(schema));
}

#[test]
fn reformat_missing_forward_relation_arguments_with_crln() {
    let schema = r#"
    generator client {
      provider = "prisma-client-js"
      output   = "../generated/client"
    }

    datasource db {
      provider = "sqlite"
      url      = "file:dev.db"
    }

    model Post {
      id     Int @id
      user   User
    }

    model User {
      id     Int @id
      posts  Post[]
    }
    "#;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client-js"
          output   = "../generated/client"
        }

        datasource db {
          provider = "sqlite"
          url      = "file:dev.db"
        }

        model Post {
          id     Int  @id
          user   User @relation(fields: [userId], references: [id])
          userId Int
        }

        model User {
          id    Int    @id
          posts Post[]
        }
    "#]];

    expected.assert_eq(&reformat(&schema.replace('\n', "\r\n")));
}
