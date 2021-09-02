use barrel::types;
use expect_test::expect;
use indoc::indoc;
use introspection_engine_tests::test_api::*;
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Sqlite))]
async fn a_one_to_one_relation_referencing_non_id(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("email", types::varchar(10).unique(true).nullable(true));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_email", types::varchar(10).unique(true).nullable(true));
                t.add_foreign_key(&["user_email"], "User", &["email"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id         Int     @id @default(autoincrement())
          user_email String? @unique(map: "sqlite_autoindex_Post_1")
          User       User?   @relation(fields: [user_email], references: [email], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id    Int     @id @default(autoincrement())
          email String? @unique(map: "sqlite_autoindex_User_1")
          Post  Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn relations_should_avoid_name_clashes(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("y", |t| {
                t.add_column("id", types::integer().primary(true));
                t.add_column("x", types::integer().nullable(false));
            });

            migration.create_table("x", |t| {
                t.add_column("id", types::integer().primary(true));
                t.add_column("y", types::integer().nullable(false));
                t.add_foreign_key(&["y"], "y", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model x {
          id     Int @id @default(autoincrement())
          y      Int
          y_xToy y   @relation(fields: [y], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model y {
          id     Int @id @default(autoincrement())
          x      Int
          x_xToy x[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn a_one_to_one_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().unique(true).nullable(true));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int   @id @default(autoincrement())
          user_id Int?  @unique(map: "sqlite_autoindex_Post_1")
          User    User? @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn one_to_one_req_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false).unique(true));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int  @id @default(autoincrement())
          user_id Int  @unique(map: "sqlite_autoindex_Post_1")
          User    User @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn two_one_to_one_relations_between_the_same_models(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("post_id", types::integer().unique(true).nullable(false));
                t.add_foreign_key(&["post_id"], "Post", &["id"]);
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().unique(true).nullable(false));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id                      Int   @id @default(autoincrement())
          user_id                 Int   @unique(map: "sqlite_autoindex_Post_1")
          User_Post_user_idToUser User  @relation("Post_user_idToUser", fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
          User_PostToUser_post_id User? @relation("PostToUser_post_id")
        }

        model User {
          id                      Int   @id @default(autoincrement())
          post_id                 Int   @unique(map: "sqlite_autoindex_User_1")
          Post_PostToUser_post_id Post  @relation("PostToUser_post_id", fields: [post_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
          Post_Post_user_idToUser Post? @relation("Post_user_idToUser")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn one_to_one_relation_on_a_singular_primary_key(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().nullable(false).unique(true));
                t.add_foreign_key(&["id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id   Int  @unique(map: "sqlite_autoindex_Post_1")
          User User @relation(fields: [id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn multiple_foreign_key_constraints_are_taken_always_in_the_same_order(api: &TestApi) -> TestResult {
    let migration = indoc! {r#"
        CREATE TABLE A
        (
            id  int NOT NULL PRIMARY KEY,
            foo int NOT NULL,
            FOREIGN KEY (foo) REFERENCES B(id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN KEY (foo) REFERENCES B(id) ON DELETE RESTRICT ON UPDATE RESTRICT
        );

        CREATE TABLE B
        (
            id int NOT NULL PRIMARY KEY
        );
    "#};

    api.database().raw_cmd(&migration).await?;

    let expected = expect![[r#"
        model A {
          id  Int @id
          foo Int
          B   B   @relation(fields: [foo], references: [id], onUpdate: Restrict)
        }

        model B {
          id Int @id
          A  A[]
        }
    "#]];

    for _ in 0..10 {
        expected.assert_eq(&api.introspect_dml().await?);
    }

    Ok(())
}
