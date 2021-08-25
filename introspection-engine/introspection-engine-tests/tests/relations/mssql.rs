use barrel::types;
use expect_test::expect;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Mssql))]
async fn two_one_to_one_relations_between_the_same_models(api: &TestApi) -> TestResult {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("post_id", types::integer().unique(true).nullable(false));

                // Other databases can't create a foreign key before the table
                // exists, SQLite can, but cannot alter table with a foreign
                // key.
                if sql_family.is_sqlite() {
                    t.add_foreign_key(&["post_id"], "Post", &["id"]);
                }
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().unique(true).nullable(false));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });

            // Other databases can't create a foreign key before the table
            // exists, SQLite can, but cannot alter table with a foreign
            // key.
            if !sql_family.is_sqlite() {
                migration.change_table("User", |t| {
                    t.add_foreign_key(&["post_id"], "Post", &["id"]);
                })
            }
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id                      Int   @id @default(autoincrement())
          user_id                 Int   @unique
          User_Post_user_idToUser User  @relation("Post_user_idToUser", fields: [user_id], references: [id], onUpdate: NoAction)
          User_PostToUser_post_id User? @relation("PostToUser_post_id")
        }

        model User {
          id                      Int   @id @default(autoincrement())
          post_id                 Int   @unique
          Post_PostToUser_post_id Post  @relation("PostToUser_post_id", fields: [post_id], references: [id], onUpdate: NoAction)
          Post_Post_user_idToUser Post? @relation("Post_user_idToUser")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
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

                t.add_foreign_key(&["user_id"], "User", &["id"]);
                t.add_foreign_key(&["post_id"], "Post", &["id"]);
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
          Post    Post @relation(fields: [post_id], references: [id], onUpdate: NoAction)
          User    User @relation(fields: [user_id], references: [id], onUpdate: NoAction)
        }

        model User {
          id           Int            @id @default(autoincrement())
          PostsToUsers PostsToUsers[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn a_one_req_to_many_relation(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().unique(false).nullable(false));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int  @id @default(autoincrement())
          user_id Int
          User    User @relation(fields: [user_id], references: [id], onUpdate: NoAction)
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn id_fields_with_foreign_key(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", move |t| {
                t.add_column("user_id", types::integer().primary(true));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          user_id Int  @id
          User    User @relation(fields: [user_id], references: [id], onUpdate: NoAction)
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn one_to_many_relation_field_names_do_not_conflict_with_many_to_many_relation_field_names(
    api: &TestApi,
) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |table| {
                table.add_column("id", types::primary());
            });

            migration.create_table("Event", |table| {
                table.add_column("id", types::primary());
                table.add_column("host_id", types::integer().nullable(false));

                table.add_foreign_key(&["host_id"], "User", &["id"]);
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
          User_Event_host_idToUser User   @relation("Event_host_idToUser", fields: [host_id], references: [id], onUpdate: NoAction)
          User_EventToUser         User[]
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

#[test_connector(tags(Mssql))]
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
          id   Int  @unique
          User User @relation(fields: [id], references: [id], onUpdate: NoAction)
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql), preview_features("NamedConstraints"))]
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
                t.add_constraint(
                    "CustomFKName",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int  @id @default(autoincrement())
          user_id Int  @unique
          User    User @relation(fields: [user_id], references: [id], onUpdate: NoAction, map: "CustomFKName")
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
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
          user_id Int  @unique
          User    User @relation(fields: [user_id], references: [id], onUpdate: NoAction)
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
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
          id     Int @id
          y      Int
          y_xToy y   @relation(fields: [y], references: [id], onUpdate: NoAction)
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

#[test_connector(tags(Mssql))]
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
                t.add_foreign_key(&["y"], "y", &["id"]);
            });

            migration.change_table("y", |t| {
                t.add_foreign_key(&["fk_x_1", "fk_x_2"], "x", &["id", "y"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model x {
          id                   Int @id @default(autoincrement())
          y                    Int
          y_x_yToy             y   @relation("x_yToy", fields: [y], references: [id], onUpdate: NoAction)
          y_xToy_fk_x_1_fk_x_2 y[] @relation("xToy_fk_x_1_fk_x_2")

          @@unique([id, y], name: "unique_y_id")
        }

        model y {
          id                   Int @id @default(autoincrement())
          x                    Int
          fk_x_1               Int
          fk_x_2               Int
          x_xToy_fk_x_1_fk_x_2 x   @relation("xToy_fk_x_1_fk_x_2", fields: [fk_x_1, fk_x_2], references: [id, y], onUpdate: NoAction)
          x_x_yToy             x[] @relation("x_yToy")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
