mod cockroachdb;
mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use barrel::types;
use expect_test::expect;
use indoc::*;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(exclude(Mssql, Mysql, Sqlite, CockroachDb))]
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
          user_id Int  @unique
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

#[test_connector(exclude(Mssql, Mysql, Sqlite, CockroachDb))]
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
          id   Int  @unique
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

#[test_connector(exclude(Mssql, Mysql, Sqlite, CockroachDb))]
async fn two_one_to_one_relations_between_the_same_models(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("post_id", types::integer().unique(true).nullable(false));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().unique(true).nullable(false));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });

            migration.change_table("User", |t| {
                t.add_foreign_key(&["post_id"], "Post", &["id"]);
            })
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id                      Int   @id @default(autoincrement())
          user_id                 Int   @unique
          User_Post_user_idToUser User  @relation("Post_user_idToUser", fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
          User_User_post_idToPost User? @relation("User_post_idToPost")
        }

        model User {
          id                      Int   @id @default(autoincrement())
          post_id                 Int   @unique
          Post_Post_user_idToUser Post? @relation("Post_user_idToUser")
          Post_User_post_idToPost Post  @relation("User_post_idToPost", fields: [post_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Sqlite, CockroachDb))]
async fn a_one_to_one_relation(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer().nullable(true));

                t.add_constraint(
                    "Post_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
                t.add_constraint("Post_user_id_key", types::unique_constraint(["user_id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int   @id @default(autoincrement())
          user_id Int?  @unique
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

#[test_connector(exclude(Sqlite, Mysql, CockroachDb))]
async fn a_one_to_one_relation_referencing_non_id(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("email", types::varchar(10).nullable(true));

                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
                t.add_constraint("User_email_key", types::unique_constraint(vec!["email"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_email", types::varchar(10).nullable(true));
                t.add_constraint(
                    "Post_user_email_fkey",
                    types::foreign_constraint(&["user_email"], "User", &["email"], None, None),
                );

                t.add_constraint("Post_pkey", types::primary_constraint(vec!["id"]));
                t.add_constraint("Post_user_email_key", types::unique_constraint(vec!["user_email"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id         Int     @id @default(autoincrement())
          user_email String? @unique @db.VarChar(10)
          User       User?   @relation(fields: [user_email], references: [email], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id    Int     @id @default(autoincrement())
          email String? @unique @db.VarChar(10)
          Post  Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Sqlite, CockroachDb))]
async fn a_one_to_many_relation(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer().unique(false).nullable(true));
                t.add_constraint(
                    "user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id      Int   @id @default(autoincrement())
          user_id Int?
          User    User? @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "user_id_fkey")
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Mssql, CockroachDb))]
async fn a_one_req_to_many_relation(api: &mut TestApi) -> TestResult {
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
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Postgres, Vitess, CockroachDb))]
async fn a_prisma_many_to_many_relation(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("_PostToUser", |t| {
                t.add_column("A", types::integer().nullable(false).unique(false));
                t.add_column("B", types::integer().nullable(false).unique(false));

                t.add_foreign_key(&["A"], "Post", &["id"]);
                t.add_foreign_key(&["B"], "User", &["id"]);

                t.add_index("test", types::index(vec!["A", "B"]).unique(true));
                t.add_index("test2", types::index(vec!["B"]).unique(false));
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Post {
            id   Int    @id @default(autoincrement())
            User User[]
        }

        model User {
            id   Int    @id @default(autoincrement())
            Post Post[]
        }
    "##};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(dm, &result);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn a_broken_prisma_many_to_many_relation(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("_PostToUser", |t| {
                t.add_column("A", types::integer().nullable(false).unique(false));
                t.add_column("B", types::integer().nullable(false).unique(false));

                t.add_foreign_key(&["A"], "Post", &["id"]);
                t.add_foreign_key(&["B"], "User", &["id"]);

                t.add_index("test", types::index(vec!["A", "B"]).unique(true));
            });
        })
        .await?;

    api.expect_re_introspect_warnings(
        indoc! {r##"
        model Post {
            id   Int    @id @default(autoincrement())
            User Author[] @relation("PostToUser")
        }

        model Author {
            id   Int    @id @default(autoincrement())
            Post Post[] @relation("PostToUser")

            @@map("User")
        }
    "##},
        expect![[r#"
            *** WARNING ***

            These models were enriched with `@@map` information taken from the previous Prisma schema:
              - "Author"
            The many-to-many relation between "Post" and "Author" is broken due to the naming of the models. Prisma creates many-to-many relations based on the alphabetical ordering of the names of the models and these two models now produce the reverse of the expected ordering."#]],
    )
    .await;

    Ok(())
}

#[test_connector(exclude(Mysql, Mssql, CockroachDb, Sqlite))]
async fn a_many_to_many_relation_with_an_id(api: &mut TestApi) -> TestResult {
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
        }

        model User {
          id           Int            @id @default(autoincrement())
          PostsToUsers PostsToUsers[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Sqlite, CockroachDb, Mssql))]
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
          User_User_direct_reportToUser       User?  @relation("User_direct_reportToUser", fields: [direct_report], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "direct_report_fkey")
          other_User_User_direct_reportToUser User[] @relation("User_direct_reportToUser")
          User_User_recruited_byToUser        User?  @relation("User_recruited_byToUser", fields: [recruited_by], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "recruited_by_fkey")
          other_User_User_recruited_byToUser  User[] @relation("User_recruited_byToUser")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

// SQLite will always make the primary key autoincrement, which makes no sense
// to build.
#[test_connector(exclude(Sqlite, Mssql, Mysql, CockroachDb))]
async fn id_fields_with_foreign_key(api: &mut TestApi) -> TestResult {
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

// SQLite cannot alter tables to add foreign keys, so skipping the tests.
#[test_connector(exclude(Sqlite, Mysql, CockroachDb))]
async fn duplicate_fks_should_ignore_one_of_them(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer().nullable(true));

                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
            });

            migration.change_table("Post", |t| {
                t.add_constraint(
                    "Post_user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            })
        })
        .await?;

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

#[test_connector(exclude(Mssql, Vitess))]
async fn prisma_1_0_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Book", |t| {
                t.add_column("id", types::custom("char(25)").primary(true));
            });

            migration.create_table("Royalty", |t| {
                t.add_column("id", types::custom("char(25)").primary(true));
            });

            migration.create_table("_BookRoyalty", |t| {
                t.add_column("id", types::custom("char(25)").primary(true));
                t.add_column("A", types::custom("char(25)").nullable(false));
                t.add_column("B", types::custom("char(25)").nullable(false));

                t.add_foreign_key(&["A"], "Book", &["id"]);
                t.add_foreign_key(&["B"], "Royalty", &["id"]);

                t.add_index("double", types::index(vec!["A", "B"]).unique(true));
                t.add_index("single", types::index(vec!["B"]).unique(false));
            });
        })
        .await?;

    let native_string = if !api.sql_family().is_sqlite() {
        "@db.Char(25)"
    } else {
        ""
    };

    let dm = formatdoc! {r##"
        model Book {{
            id        String      @id {native_string}
            Royalty   Royalty[]   @relation("BookRoyalty")
        }}

        model Royalty {{
            id        String      @id {native_string}
            Book      Book[]      @relation("BookRoyalty")
        }}
    "##};

    let result = api.introspect().await?;
    api.assert_eq_datamodels(&dm, &result);

    Ok(())
}

#[test_connector(exclude(Mysql, Sqlite, Mssql))]
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
          id       Int @id
          y        Int
          y_x_yToy y   @relation("x_yToy", fields: [y], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model y {
          id       Int @id
          x        Int
          x_x_yToy x[] @relation("x_yToy")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Mssql, CockroachDb))]
async fn one_to_many_relation_field_names_do_not_conflict_with_many_to_many_relation_field_names(
    api: &mut TestApi,
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
          User_EventToUser         User[] @relation("EventToUser")
        }

        model User {
          id                        Int     @id @default(autoincrement())
          Event_Event_host_idToUser Event[] @relation("Event_host_idToUser")
          Event_EventToUser         Event[] @relation("EventToUser")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite, Mssql, Mysql, CockroachDb))]
async fn one_to_one_req_relation_with_custom_fk_name(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true).nullable(false));
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_key", types::index(["user_id"]).unique(true));
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
          User    User @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "CustomFKName")
        }

        model User {
          id   Int   @id @default(autoincrement())
          Post Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
