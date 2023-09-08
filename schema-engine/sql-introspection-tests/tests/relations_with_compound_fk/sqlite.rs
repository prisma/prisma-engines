use barrel::types;
use expect_test::expect;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Sqlite))]
async fn compound_foreign_keys_for_duplicate_one_to_many_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_constraint("sqlite_autoindex_User_1", types::unique_constraint(["id", "age"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));
                t.add_column("other_user_id", types::integer().nullable(true));
                t.add_column("other_user_age", types::integer().nullable(true));

                t.add_foreign_key(&["user_id", "user_age"], "User", &["id", "age"]);
                t.add_foreign_key(&["other_user_id", "other_user_age"], "User", &["id", "age"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id                                           Int   @id @default(autoincrement())
          user_id                                      Int?
          user_age                                     Int?
          other_user_id                                Int?
          other_user_age                               Int?
          User_Post_other_user_id_other_user_ageToUser User? @relation("Post_other_user_id_other_user_ageToUser", fields: [other_user_id, other_user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
          User_Post_user_id_user_ageToUser             User? @relation("Post_user_id_user_ageToUser", fields: [user_id, user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id                                           Int    @id @default(autoincrement())
          age                                          Int
          Post_Post_other_user_id_other_user_ageToUser Post[] @relation("Post_other_user_id_other_user_ageToUser")
          Post_Post_user_id_user_ageToUser             Post[] @relation("Post_user_id_user_ageToUser")

          @@unique([id, age], map: "sqlite_autoindex_User_1")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn compound_foreign_keys_for_one_to_many_relations_with_non_unique_index(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());

                t.add_constraint("sqlite_autoindex_User_1", types::unique_constraint(vec!["id", "age"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_foreign_key(&["user_id", "user_age"], "User", &["id", "age"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id       Int  @id @default(autoincrement())
          user_id  Int
          user_age Int
          User     User @relation(fields: [user_id, user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   Int    @id @default(autoincrement())
          age  Int
          Post Post[]

          @@unique([id, age], map: "sqlite_autoindex_User_1")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn compound_foreign_keys_for_one_to_one_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());

                t.add_index("user_unique", types::index(vec!["id", "age"]).unique(true));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));

                t.add_foreign_key(&["user_id", "user_age"], "User", &["id", "age"]);

                t.add_constraint(
                    "sqlite_autoindex_Post_1",
                    types::unique_constraint(vec!["user_id", "user_age"]),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id       Int   @id @default(autoincrement())
          user_id  Int?
          user_age Int?
          User     User? @relation(fields: [user_id, user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)

          @@unique([user_id, user_age], map: "sqlite_autoindex_Post_1")
        }

        model User {
          id   Int   @id @default(autoincrement())
          age  Int
          Post Post?

          @@unique([id, age], map: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn compound_foreign_keys_for_required_one_to_one_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());

                t.add_index("user_unique", types::index(vec!["id", "age"]).unique(true));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_foreign_key(&["user_id", "user_age"], "User", &["id", "age"]);

                t.add_constraint(
                    "sqlite_autoindex_Post_1",
                    types::unique_constraint(vec!["user_id", "user_age"]),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id       Int  @id @default(autoincrement())
          user_id  Int
          user_age Int
          User     User @relation(fields: [user_id, user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)

          @@unique([user_id, user_age], map: "sqlite_autoindex_Post_1")
        }

        model User {
          id   Int   @id @default(autoincrement())
          age  Int
          Post Post?

          @@unique([id, age], map: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn compound_foreign_keys_for_required_self_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer());
                t.add_column("partner_age", types::integer());

                t.add_foreign_key(&["partner_id", "partner_age"], "Person", &["id", "age"]);
                t.add_constraint("doesnt_matter", types::unique_constraint(vec!["id", "age"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Person {
          id           Int      @id @default(autoincrement())
          age          Int
          partner_id   Int
          partner_age  Int
          Person       Person   @relation("PersonToPerson", fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
          other_Person Person[] @relation("PersonToPerson")

          @@unique([id, age], map: "sqlite_autoindex_Person_1")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn compound_foreign_keys_for_self_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer().nullable(true));
                t.add_column("partner_age", types::integer().nullable(true));

                t.add_foreign_key(&["partner_id", "partner_age"], "Person", &["id", "age"]);
                t.add_constraint("post_user_unique", types::unique_constraint(vec!["id", "age"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Person {
          id           Int      @id @default(autoincrement())
          age          Int
          partner_id   Int?
          partner_age  Int?
          Person       Person?  @relation("PersonToPerson", fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
          other_Person Person[] @relation("PersonToPerson")

          @@unique([id, age], map: "sqlite_autoindex_Person_1")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn compound_foreign_keys_with_defaults(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer().default(0));
                t.add_column("partner_age", types::integer().default(0));

                t.add_constraint("post_user_unique", types::unique_constraint(vec!["id", "age"]));
                t.add_foreign_key(&["partner_id", "partner_age"], "Person", &["id", "age"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Person {
          id           Int      @id @default(autoincrement())
          age          Int
          partner_id   Int      @default(0)
          partner_age  Int      @default(0)
          Person       Person   @relation("PersonToPerson", fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
          other_Person Person[] @relation("PersonToPerson")

          @@unique([id, age], map: "sqlite_autoindex_Person_1")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn compound_foreign_keys_for_one_to_many_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("age", types::integer());

                t.add_index("user_unique", types::index(vec!["id", "age"]).unique(true));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));

                t.add_constraint(
                    "Post_fk",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id       Int   @id @default(autoincrement())
          user_id  Int?
          user_age Int?
          User     User? @relation(fields: [user_id, user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   Int    @id @default(autoincrement())
          age  Int
          Post Post[]

          @@unique([id, age], map: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn compound_foreign_keys_for_one_to_many_relations_with_mixed_requiredness(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());

                t.add_index("user_unique", types::index(vec!["id", "age"]).unique(true));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_column("user_age", types::integer().nullable(true));

                t.add_constraint(
                    "Post_user_id_user_age",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id       Int   @id @default(autoincrement())
          user_id  Int
          user_age Int?
          User     User? @relation(fields: [user_id, user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id   Int    @id @default(autoincrement())
          age  Int
          Post Post[]

          @@unique([id, age], map: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
