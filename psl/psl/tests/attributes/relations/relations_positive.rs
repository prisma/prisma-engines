use crate::{common::*, with_header, Provider};
use psl::parser_database::ScalarType;

#[test]
fn settings_must_be_deteced() {
    let dml = indoc! {r#"
        model Todo {
          id       Int  @id
          parentId Int?

          child_todos Todo[] @relation("MyRelation")
          parent_todo Todo? @relation("MyRelation", fields: parentId, references: id)
        }
    "#};

    let schema = parse_schema(dml);

    let todo_model = schema.assert_has_model("Todo");
    todo_model
        .assert_has_relation_field("parent_todo")
        .assert_relation_to(todo_model.id);
}

#[test]
fn resolve_relation() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String
          posts Post[]
        }

        model Post {
          id     Int    @id
          text   String
          userId Int

          user User @relation(fields: [userId], references: [id])
        }
    "#};

    let schema = parse_schema(dml);
    let user_model = schema.assert_has_model("User");
    let post_model = schema.assert_has_model("Post");
    user_model.assert_has_scalar_field("firstName");
    user_model
        .assert_has_relation_field("posts")
        .assert_relation_to(post_model.id);

    post_model.assert_has_scalar_field("text");
    post_model
        .assert_has_relation_field("user")
        .assert_relation_to(user_model.id);
}

#[test]
fn resolve_related_field() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String @unique
          posts Post[]
        }

        model Post {
          id            Int    @id
          text          String
          userFirstName String
          user          User   @relation(fields: [userFirstName], references: [firstName])
        }
    "#};

    let schema = parse_schema(dml);

    let user_model = schema.assert_has_model("User");
    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_has_relation_field("user")
        .assert_relation_to(user_model.id);
}

#[test]
fn resolve_related_fields() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String
          lastName String
          posts Post[]

          @@unique([firstName, lastName])
        }

        model Post {
          id Int @id
          text String
          authorFirstName String
          authorLastName  String
          user            User @relation(fields: [authorFirstName, authorLastName], references: [firstName, lastName])
        }
    "#};

    let schema = parse_schema(dml);

    let user_model = schema.assert_has_model("User");
    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_has_relation_field("user")
        .assert_relation_to(user_model.id);
}

#[test]
fn allow_multiple_relations() {
    let dml = indoc! {r#"
        model User {
          id         Int    @id
          more_posts Post[] @relation(name: "more_posts")
          posts      Post[]
        }

        model Post {
          id            Int    @id
          text          String
          userId        Int
          postingUserId Int

          user         User   @relation(fields: userId, references: id)
          posting_user User   @relation(name: "more_posts", fields: postingUserId, references: id)
        }
    "#};

    let schema = parse_schema(dml);
    let user_model = schema.assert_has_model("User");
    let post_model = schema.assert_has_model("Post");
    user_model
        .assert_field_count(3)
        .assert_has_relation_field("posts")
        .assert_relation_to(post_model.id);
    user_model
        .assert_has_relation_field("more_posts")
        .assert_relation_to(post_model.id);

    post_model
        .assert_field_count(6)
        .assert_has_scalar_field("text")
        .assert_scalar_type(ScalarType::String);
    post_model
        .assert_has_relation_field("user")
        .assert_relation_to(user_model.id);
    post_model
        .assert_has_relation_field("posting_user")
        .assert_relation_to(user_model.id);
}

#[test]
fn allow_complicated_self_relations() {
    let dml = indoc! {r#"
        model User {
          id     Int  @id
          sonId  Int? @unique
          wifeId Int? @unique

          son     User? @relation(name: "offspring", fields: sonId, references: id)
          father  User? @relation(name: "offspring")

          husband User? @relation(name: "spouse")
          wife    User? @relation(name: "spouse", fields: wifeId, references: id)
        }
    "#};

    let schema = parse_schema(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_relation_field("son")
        .assert_relation_to(user_model.id);
    user_model
        .assert_has_relation_field("father")
        .assert_relation_to(user_model.id);
    user_model
        .assert_has_relation_field("husband")
        .assert_relation_to(user_model.id);
    user_model
        .assert_has_relation_field("wife")
        .assert_relation_to(user_model.id);
}

#[test]
fn allow_explicit_fk_name_definition() {
    let dml = indoc! {r#"
        model User {
          user_id Int    @id
          posts   Post[]
        }

        model Post {
          post_id Int    @id
          user_id Int
          user    User    @relation(fields: user_id, references: user_id, map: "CustomFKName")
        }
    "#};

    let schema = parse_schema(&with_header(dml, Provider::Postgres, &[]));

    schema.assert_has_model("User").assert_has_relation_field("posts");
    schema.assert_has_model("Post").assert_has_relation_field("user");
}

#[test]
fn one_to_one_optional() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          b  B?
        }

        model B {
          id   Int  @id
          a_id Int? @unique
          a    A?   @relation(fields: [a_id], references: [id])
        }
    "#};

    let schema = parse_schema(dml);
    schema.assert_has_model("A").assert_has_relation_field("b");
    schema.assert_has_model("B").assert_has_relation_field("a");
}

#[test]
fn embedded_many_to_many_relations_work_on_mongodb() {
    let dml = indoc! {r#"
        model A {
          id    String   @id @map("_id") @default(auto()) @test.ObjectId
          b_ids String[] @test.ObjectId
          bs    B[]      @relation(fields: [b_ids], references: [id])
        }

        model B {
          id    String   @id @map("_id") @default(auto()) @test.ObjectId
          a_ids String[] @test.ObjectId
          as    A[]      @relation(fields: [a_ids], references: [id])
        }
    "#};

    let schema = parse_schema(&with_header(dml, Provider::Mongo, &[]));

    schema.assert_has_model("A").assert_has_relation_field("bs");
    schema.assert_has_model("B").assert_has_relation_field("as");
}

#[test]
fn implicit_many_to_many_relations_work_on_postgresql() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          bs B[] @relation("foo")
        }

        model B {
          id Int @id
          as A[] @relation("foo")
        }
    "#};

    let schema = parse_schema(&with_header(dml, Provider::Postgres, &[]));
    schema.assert_has_model("A").assert_has_relation_field("bs");
    schema.assert_has_model("B").assert_has_relation_field("as");
}

#[test]
fn implicit_many_to_many_relations_work_on_mysql() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          bs B[] @relation("foo")
        }

        model B {
          id Int @id
          as A[] @relation("foo")
        }
    "#};

    let schema = parse_schema(&with_header(dml, Provider::Mysql, &[]));
    schema.assert_has_model("A").assert_has_relation_field("bs");
    schema.assert_has_model("B").assert_has_relation_field("as");
}

#[test]
fn implicit_many_to_many_relations_work_on_sql_server() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          bs B[] @relation("foo")
        }

        model B {
          id Int @id
          as A[] @relation("foo")
        }
    "#};

    let schema = parse_schema(&with_header(dml, Provider::SqlServer, &[]));
    schema.assert_has_model("A").assert_has_relation_field("bs");
    schema.assert_has_model("B").assert_has_relation_field("as");
}

#[test]
fn implicit_many_to_many_relations_work_on_sqlite() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          bs B[] @relation("foo")
        }

        model B {
          id Int @id
          as A[] @relation("foo")
        }
    "#};

    let schema = parse_schema(&with_header(dml, Provider::Sqlite, &[]));
    schema.assert_has_model("A").assert_has_relation_field("bs");
    schema.assert_has_model("B").assert_has_relation_field("as");
}

#[test]
fn implicit_many_to_many_relations_work_on_cockroach() {
    let dml = indoc! {r#"
        model A {
          id Int @id
          bs B[] @relation("foo")
        }

        model B {
          id Int @id
          as A[] @relation("foo")
        }
    "#};

    let schema = parse_schema(&with_header(dml, Provider::Cockroach, &["cockroachDb"]));
    schema.assert_has_model("A").assert_has_relation_field("bs");
    schema.assert_has_model("B").assert_has_relation_field("as");
}
