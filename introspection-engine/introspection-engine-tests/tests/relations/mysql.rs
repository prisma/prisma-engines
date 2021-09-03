use barrel::types;
use expect_test::expect;
use indoc::indoc;
use introspection_engine_tests::test_api::*;
use quaint::prelude::Queryable;
use test_macros::test_connector;

#[test_connector(tags(Mysql))]
async fn a_many_to_many_relation_with_an_id(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("PostsToUsers", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_column("post_id", types::integer().nullable(false));

                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
                t.inject_custom(
                    "CONSTRAINT post_id FOREIGN KEY (post_id) REFERENCES `Post`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id           Int            @id @default(autoincrement())
          PostsToUsers PostsToUsers[]
        }

        model PostsToUsers {
          id      Int  @id @default(autoincrement())
          user_id Int
          post_id Int
          Post    Post @relation(fields: [post_id], references: [id], map: "post_id")
          User    User @relation(fields: [user_id], references: [id], map: "user_id")

          @@index([post_id], map: "post_id")
          @@index([user_id], map: "user_id")
        }

        model User {
          id           Int            @id @default(autoincrement())
          PostsToUsers PostsToUsers[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn a_one_req_to_many_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().unique(false).nullable(false));
                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int  @id @default(autoincrement())
          user_id Int
          User    User @relation(fields: [user_id], references: [id], map: "user_id")

          @@index([user_id], map: "user_id")
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn a_one_to_many_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().unique(false).nullable(true));
                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE SET NULL ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int   @id @default(autoincrement())
          user_id Int?
          User    User? @relation(fields: [user_id], references: [id], map: "user_id")

          @@index([user_id], map: "user_id")
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn a_self_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("recruited_by", types::integer().nullable(true));
                t.add_column("direct_report", types::integer().nullable(true));

                t.inject_custom(
                    "CONSTRAINT recruited_by FOREIGN KEY (recruited_by) REFERENCES `User`(id) ON DELETE SET NULL ON UPDATE CASCADE",
                );
                t.inject_custom(
                    "CONSTRAINT direct_report FOREIGN KEY (direct_report) REFERENCES `User`(id) ON DELETE SET NULL ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model User {
          id                                  Int    @id @default(autoincrement())
          recruited_by                        Int?
          direct_report                       Int?
          User_UserToUser_direct_report       User?  @relation("UserToUser_direct_report", fields: [direct_report], references: [id], map: "direct_report")
          User_UserToUser_recruited_by        User?  @relation("UserToUser_recruited_by", fields: [recruited_by], references: [id], map: "recruited_by")
          other_User_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
          other_User_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")

          @@index([direct_report], map: "direct_report")
          @@index([recruited_by], map: "recruited_by")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn duplicate_fks_should_ignore_one_of_them(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(true));
                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE SET NULL ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int   @id @default(autoincrement())
          user_id Int?
          User    User? @relation(fields: [user_id], references: [id], map: "user_id")

          @@index([user_id], map: "user_id")
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn one_to_many_relation_field_names_do_not_conflict_with_many_to_many_relation_field_names(
    api: &TestApi,
) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |table| {
                table.add_column("id", types::integer().increments(true));
                table.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Event", |table| {
                table.add_column("id", types::integer().increments(true));
                table.add_column("host_id", types::integer().nullable(false));

                table.inject_custom(
                    "CONSTRAINT host_id FOREIGN KEY (host_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
                table.add_constraint("Event_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("_EventToUser", |table| {
                table.add_column("A", types::integer().nullable(false));
                table.add_column("B", types::integer().nullable(false));

                table.add_foreign_key(&["A"], "Event", &["id"]);
                table.add_foreign_key(&["B"], "User", &["id"]);

                table.add_index(
                    "_EventToUser_AB_unique",
                    barrel::types::index(vec!["A", "B"]).unique(true),
                );

                table.add_index("_EventToUser_B_index", barrel::types::index(vec!["B"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Event {
          id                       Int    @id @default(autoincrement())
          host_id                  Int
          User_Event_host_idToUser User   @relation("Event_host_idToUser", fields: [host_id], references: [id], map: "host_id")
          User_EventToUser         User[]

          @@index([host_id], map: "host_id")
        }

        model User {
          id                        Int     @id @default(autoincrement())
          Event_Event_host_idToUser Event[] @relation("Event_host_idToUser")
          Event_EventToUser         Event[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
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
                t.inject_custom("CONSTRAINT y FOREIGN KEY (y) REFERENCES `y`(id) ON DELETE RESTRICT ON UPDATE CASCADE");
            });
        })
        .await?;

    let expected = expect![[r#"
        model x {
          id     Int @id
          y      Int
          y_xToy y   @relation(fields: [y], references: [id], map: "y")

          @@index([y], map: "y")
        }

        model y {
          id     Int @id
          x      Int
          x_xToy x[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn relations_should_avoid_name_clashes_2(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("x", move |t| {
                t.add_column("id", types::primary());
                t.add_column("y", types::integer().nullable(false));
                t.add_index("unique_y_id", types::index(vec!["id", "y"]).unique(true));
            });

            migration.create_table("y", move |t| {
                t.add_column("id", types::primary());
                t.add_column("x", types::integer().nullable(false));
                t.add_column("fk_x_1", types::integer().nullable(false));
                t.add_column("fk_x_2", types::integer().nullable(false));
            });

            migration.change_table("x", |t| {
                t.inject_custom(
                    "ADD CONSTRAINT y FOREIGN KEY (y) REFERENCES `y`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });

            migration.change_table("y", |t| {
                t.inject_custom(
                    "ADD CONSTRAINT fk_x_1 FOREIGN KEY (fk_x_1, fk_x_2) REFERENCES `x`(id, y) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model x {
          id                   Int @id @default(autoincrement())
          y                    Int
          y_x_yToy             y   @relation("x_yToy", fields: [y], references: [id], map: "y")
          y_xToy_fk_x_1_fk_x_2 y[] @relation("xToy_fk_x_1_fk_x_2")

          @@unique([id, y], map: "unique_y_id")
          @@index([y], map: "y")
        }

        model y {
          id                   Int @id @default(autoincrement())
          x                    Int
          fk_x_1               Int
          fk_x_2               Int
          x_xToy_fk_x_1_fk_x_2 x   @relation("xToy_fk_x_1_fk_x_2", fields: [fk_x_1, fk_x_2], references: [id, y], map: "fk_x_1")
          x_x_yToy             x[] @relation("x_yToy")

          @@index([fk_x_1, fk_x_2], map: "fk_x_1")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn two_one_to_one_relations_between_the_same_models(api: &TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("post_id", types::integer().unique(true).nullable(false));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().unique(true).nullable(false));

                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });

            // Other databases can't create a foreign key before the table
            // exists, SQLite can, but cannot alter table with a foreign
            // key.
            if !sql_family.is_sqlite() {
                migration.change_table("User", |t| {
                    t.inject_custom(
                        "ADD CONSTRAINT post_id FOREIGN KEY (post_id) REFERENCES `Post`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                    );
                })
            }
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id                      Int   @id @default(autoincrement())
          user_id                 Int   @unique(map: "user_id")
          User_Post_user_idToUser User  @relation("Post_user_idToUser", fields: [user_id], references: [id], map: "user_id")
          User_PostToUser_post_id User? @relation("PostToUser_post_id")
        }

        model User {
          id                      Int   @id @default(autoincrement())
          post_id                 Int   @unique(map: "post_id")
          Post_PostToUser_post_id Post  @relation("PostToUser_post_id", fields: [post_id], references: [id], map: "post_id")
          Post_Post_user_idToUser Post? @relation("Post_user_idToUser")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn a_one_to_one_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().unique(true).nullable(true));
                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE SET NULL ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int   @id @default(autoincrement())
          user_id Int?  @unique(map: "user_id")
          User    User? @relation(fields: [user_id], references: [id], map: "user_id")
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
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

                t.inject_custom(
                    "CONSTRAINT user_email FOREIGN KEY (user_email) REFERENCES `User`(email) ON DELETE SET NULL ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id         Int     @id @default(autoincrement())
          user_email String? @unique(map: "user_email") @db.VarChar(10)
          User       User?   @relation(fields: [user_email], references: [email], map: "user_email")
        }

        model User {
          id    Int     @id @default(autoincrement())
          email String? @unique(map: "email") @db.VarChar(10)
          Post  Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

// SQLite will always make the primary key autoincrement, which makes no sense
// to build.
#[test_connector(tags(Mysql))]
async fn id_fields_with_foreign_key(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", move |t| {
                t.add_column("user_id", types::integer().primary(true));
                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          user_id Int  @id
          User    User @relation(fields: [user_id], references: [id], map: "user_id")
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn one_to_one_req_relation_with_custom_fk_name(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(&["id"]));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(&["id"]));
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_key", types::index(&["user_id"]).unique(true));
                t.inject_custom(
                    "CONSTRAINT CustomFKName FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int  @id @default(autoincrement())
          user_id Int  @unique
          User    User @relation(fields: [user_id], references: [id], map: "CustomFKName")
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
#[test_connector(tags(Mysql))]
async fn one_to_one_req_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false).unique(true));
                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int  @id @default(autoincrement())
          user_id Int  @unique(map: "user_id")
          User    User @relation(fields: [user_id], references: [id], map: "user_id")
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn one_to_one_relation_on_a_singular_primary_key(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().nullable(false).unique(true));
                t.inject_custom(
                    "CONSTRAINT id FOREIGN KEY (id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id   Int  @unique(map: "id")
          User User @relation(fields: [id], references: [id], map: "id")
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql57))]
async fn multiple_foreign_key_constraints_are_taken_always_in_the_same_order(api: &TestApi) -> TestResult {
    let migration = indoc! {r#"
        CREATE TABLE A
        (
            id  int primary key,
            foo int not null
        );

        CREATE TABLE B
        (
            id int primary key
        );

        ALTER TABLE A ADD CONSTRAINT fk_1 FOREIGN KEY (foo) REFERENCES B(id) ON DELETE CASCADE ON UPDATE CASCADE;
        ALTER TABLE A ADD CONSTRAINT fk_2 FOREIGN KEY (foo) REFERENCES B(id) ON DELETE RESTRICT ON UPDATE RESTRICT;
    "#};

    api.database().raw_cmd(migration).await?;

    let expected = expect![[r#"
        model A {
          id  Int @id
          foo Int
          B   B   @relation(fields: [foo], references: [id], onDelete: Cascade, map: "fk_1")

          @@index([foo], map: "fk_2")
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
