use barrel::types;
use expect_test::expect;
use indoc::indoc;
use quaint::prelude::Queryable;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Sqlite))]
async fn a_many_to_many_relation_with_an_id(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE "User" (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE "Post" (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE "PostsToUsers" (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES "User"("id"),
            post_id INTEGER NOT NULL REFERENCES "Post"("id")
        );
    "#;
    api.raw_cmd(sql).await;

    let expected = expect![[r#"
        model Post {
          id           Int            @id @default(autoincrement())
          PostsToUsers PostsToUsers[]
        }

        model PostsToUsers {
          id      Int  @id @default(autoincrement())
          user_id Int
          post_id Int
          Post    Post @relation(fields: [post_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
          User    User @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id           Int            @id @default(autoincrement())
          PostsToUsers PostsToUsers[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn a_one_to_one_relation_referencing_non_id(api: &mut TestApi) -> TestResult {
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
async fn relations_should_avoid_name_clashes(api: &mut TestApi) -> TestResult {
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
          id       Int @id @default(autoincrement())
          y        Int
          y_x_yToy y   @relation("x_yToy", fields: [y], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model y {
          id       Int @id @default(autoincrement())
          x        Int
          x_x_yToy x[] @relation("x_yToy")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn a_one_to_one_relation(api: &mut TestApi) -> TestResult {
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
async fn one_to_one_req_relation(api: &mut TestApi) -> TestResult {
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
async fn two_one_to_one_relations_between_the_same_models(api: &mut TestApi) -> TestResult {
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
          User_User_post_idToPost User? @relation("User_post_idToPost")
        }

        model User {
          id                      Int   @id @default(autoincrement())
          post_id                 Int   @unique(map: "sqlite_autoindex_User_1")
          Post_Post_user_idToUser Post? @relation("Post_user_idToUser")
          Post_User_post_idToPost Post  @relation("User_post_idToPost", fields: [post_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn one_to_one_relation_on_a_singular_primary_key(api: &mut TestApi) -> TestResult {
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
async fn multiple_foreign_key_constraints_are_taken_always_in_the_same_order(api: &mut TestApi) -> TestResult {
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

    api.database().raw_cmd(migration).await?;

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

    expected.assert_eq(&api.introspect_dml().await?);
    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn a_self_relation(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("recruited_by", types::integer().nullable(true));
                t.add_column("direct_report", types::integer().nullable(true));

                t.add_constraint(
                    "recruited_by_fkey",
                    types::foreign_constraint(&["recruited_by"], "User", &["id"], None, None),
                );
                t.add_constraint(
                    "direct_report_fkey",
                    types::foreign_constraint(&["direct_report"], "User", &["id"], None, None),
                );
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model User {
          id                                  Int    @id @default(autoincrement())
          recruited_by                        Int?
          direct_report                       Int?
          User_User_direct_reportToUser       User?  @relation("User_direct_reportToUser", fields: [direct_report], references: [id], onDelete: NoAction, onUpdate: NoAction)
          other_User_User_direct_reportToUser User[] @relation("User_direct_reportToUser")
          User_User_recruited_byToUser        User?  @relation("User_recruited_byToUser", fields: [recruited_by], references: [id], onDelete: NoAction, onUpdate: NoAction)
          other_User_User_recruited_byToUser  User[] @relation("User_recruited_byToUser")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn a_one_to_many_relation(api: &mut TestApi) -> TestResult {
    let sql = r#"
        CREATE TABLE "User" (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE "Post" (
            id INTEGER PRIMARY KEY,
            user_id INTEGER,
            CONSTRAINT "user_id_fkey" FOREIGN KEY (user_id) REFERENCES "User"(id)
        );
    "#;
    api.raw_cmd(sql).await;

    let expected = expect![[r#"
        model Post {
          id      Int   @id @default(autoincrement())
          user_id Int?
          User    User? @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn relations_should_avoid_name_clashes_2(api: &mut TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("x", move |t| {
                t.add_column("id", types::primary());
                t.add_column("y", types::integer().nullable(false));
                t.add_index("unique_y_id", types::index(vec!["id", "y"]).unique(true));

                if sql_family.is_sqlite() {
                    t.add_foreign_key(&["y"], "y", &["id"]);
                }
            });

            migration.create_table("y", move |t| {
                t.add_column("id", types::primary());
                t.add_column("x", types::integer().nullable(false));
                t.add_column("fk_x_1", types::integer().nullable(false));
                t.add_column("fk_x_2", types::integer().nullable(false));

                if sql_family.is_sqlite() {
                    t.add_foreign_key(&["fk_x_1", "fk_x_2"], "x", &["id", "y"]);
                }
            });

            if !sql_family.is_sqlite() {
                migration.change_table("x", |t| {
                    t.add_foreign_key(&["y"], "y", &["id"]);
                });

                migration.change_table("y", |t| {
                    t.add_constraint(
                        "y_fkey",
                        types::foreign_constraint(&["fk_x_1", "fk_x_2"], "x", &["id", "y"], None, None),
                    );
                });
            }
        })
        .await?;

    let expected = expect![[r#"
        model x {
          id                   Int @id @default(autoincrement())
          y                    Int
          y_x_yToy             y   @relation("x_yToy", fields: [y], references: [id], onDelete: NoAction, onUpdate: NoAction)
          y_y_fk_x_1_fk_x_2Tox y[] @relation("y_fk_x_1_fk_x_2Tox")

          @@unique([id, y], map: "unique_y_id")
        }

        model y {
          id                   Int @id @default(autoincrement())
          x                    Int
          fk_x_1               Int
          fk_x_2               Int
          x_x_yToy             x[] @relation("x_yToy")
          x_y_fk_x_1_fk_x_2Tox x   @relation("y_fk_x_1_fk_x_2Tox", fields: [fk_x_1, fk_x_2], references: [id, y], onDelete: NoAction, onUpdate: NoAction)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
