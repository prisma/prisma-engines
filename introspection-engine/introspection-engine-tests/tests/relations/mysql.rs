use barrel::types;
use expect_test::expect;
use introspection_engine_tests::test_api::*;
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
          Post    Post @relation(fields: [post_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
          User    User @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@index([post_id], name: "post_id")
          @@index([user_id], name: "user_id")
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
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int  @id @default(autoincrement())
          user_id Int
          User    User @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@index([user_id], name: "user_id")
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
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int   @id @default(autoincrement())
          user_id Int?
          User    User? @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@index([user_id], name: "user_id")
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

                t.add_foreign_key(&["recruited_by"], "User", &["id"]);
                t.add_foreign_key(&["direct_report"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model User {
          id                                  Int    @id @default(autoincrement())
          recruited_by                        Int?
          direct_report                       Int?
          User_UserToUser_direct_report       User?  @relation("UserToUser_direct_report", fields: [direct_report], references: [id], onDelete: NoAction, onUpdate: NoAction)
          User_UserToUser_recruited_by        User?  @relation("UserToUser_recruited_by", fields: [recruited_by], references: [id], onDelete: NoAction, onUpdate: NoAction)
          other_User_UserToUser_direct_report User[] @relation("UserToUser_direct_report")
          other_User_UserToUser_recruited_by  User[] @relation("UserToUser_recruited_by")

          @@index([direct_report], name: "direct_report")
          @@index([recruited_by], name: "recruited_by")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

// SQLite cannot alter tables to add foreign keys, so skipping the tests.
#[test_connector(tags(Mysql), exclude(Sqlite))]
async fn duplicate_fks_should_ignore_one_of_them(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(true));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });

            migration.change_table("Post", |t| {
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            })
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int   @id @default(autoincrement())
          user_id Int?
          User    User? @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@index([user_id], name: "user_id")
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
          User_Event_host_idToUser User   @relation("Event_host_idToUser", fields: [host_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
          User_EventToUser         User[]

          @@index([host_id], name: "host_id")
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
                t.add_foreign_key(&["y"], "y", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model x {
          id     Int @id
          y      Int
          y_xToy y   @relation(fields: [y], references: [id], onDelete: NoAction, onUpdate: NoAction)

          @@index([y], name: "y")
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
          y_x_yToy             y   @relation("x_yToy", fields: [y], references: [id], onDelete: NoAction, onUpdate: NoAction)
          y_xToy_fk_x_1_fk_x_2 y[] @relation("xToy_fk_x_1_fk_x_2")

          @@unique([id, y], name: "unique_y_id")
          @@index([y], name: "y")
        }

        model y {
          id                   Int @id @default(autoincrement())
          x                    Int
          fk_x_1               Int
          fk_x_2               Int
          x_xToy_fk_x_1_fk_x_2 x   @relation("xToy_fk_x_1_fk_x_2", fields: [fk_x_1, fk_x_2], references: [id, y], onDelete: NoAction, onUpdate: NoAction)
          x_x_yToy             x[] @relation("x_yToy")

          @@index([fk_x_1, fk_x_2], name: "fk_x_1")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
