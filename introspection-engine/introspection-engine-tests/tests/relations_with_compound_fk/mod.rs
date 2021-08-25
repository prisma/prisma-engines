mod mssql;
mod mysql;
mod sqlite;

use barrel::types;
use expect_test::expect;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(exclude(Sqlite, Mysql))]
async fn compound_foreign_keys_for_one_to_one_relations(api: &TestApi) -> TestResult {
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
                    "post_user_unique",
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

          @@unique([user_id, user_age], name: "post_user_unique")
        }

        model User {
          id   Int   @id @default(autoincrement())
          age  Int
          Post Post?

          @@unique([id, age], name: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite, Mssql, Mysql))]
async fn compound_foreign_keys_for_required_one_to_one_relations(api: &TestApi) -> TestResult {
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
                    "post_user_unique",
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

          @@unique([user_id, user_age], name: "post_user_unique")
        }

        model User {
          id   Int   @id @default(autoincrement())
          age  Int
          Post Post?

          @@unique([id, age], name: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql))]
async fn compound_foreign_keys_for_one_to_many_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
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

          @@unique([id, age], name: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Mssql))]
async fn compound_foreign_keys_for_one_to_many_relations_with_mixed_requiredness(api: &TestApi) -> TestResult {
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

                t.add_foreign_key(&["user_id", "user_age"], "User", &["id", "age"]);
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

          @@unique([id, age], name: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Mssql))]
async fn compound_foreign_keys_for_required_one_to_many_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());

                t.add_index("user_unique", types::index(vec!["id", "age"]).unique(true));
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

          @@unique([id, age], name: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Sqlite, Mysql, Mssql))]
async fn compound_foreign_keys_for_required_self_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer());
                t.add_column("partner_age", types::integer());

                t.add_foreign_key(&["partner_id", "partner_age"], "Person", &["id", "age"]);
                t.add_constraint("post_user_unique", types::unique_constraint(vec!["id", "age"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Person {
          id           Int      @id @default(autoincrement())
          age          Int
          partner_id   Int
          partner_age  Int
          Person       Person   @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
          other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")

          @@unique([id, age], name: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Sqlite))]
async fn compound_foreign_keys_for_self_relations(api: &TestApi) -> TestResult {
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
          Person       Person?  @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
          other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")

          @@unique([id, age], name: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Sqlite, Mssql))]
async fn compound_foreign_keys_with_defaults(api: &TestApi) -> TestResult {
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
          Person       Person   @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)
          other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")

          @@unique([id, age], name: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Sqlite, Mssql))]
async fn compound_foreign_keys_for_one_to_many_relations_with_non_unique_index(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());

                t.add_constraint("post_user_unique", types::unique_constraint(vec!["id", "age"]));
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

          @@unique([id, age], name: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Mssql))]
async fn repro_matt_references_on_wrong_side(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));
                t.set_primary_key(&["one", "two"]);
            });

            migration.create_table("b", |t| {
                t.add_column("id", types::primary());
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));
                t.add_foreign_key(&["one", "two"], "a", &["one", "two"])
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          one Int
          two Int
          b   b[]

          @@id([one, two])
        }

        model b {
          id  Int @id @default(autoincrement())
          one Int
          two Int
          a   a   @relation(fields: [one, two], references: [one, two], onDelete: NoAction, onUpdate: NoAction)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Mssql))]
async fn a_compound_fk_pk_with_overlapping_primary_key(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));

                t.set_primary_key(&["one", "two"]);
            });
            migration.create_table("b", |t| {
                t.add_column("dummy", types::integer().nullable(false));
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));

                t.add_foreign_key(&["one", "two"], "a", &["one", "two"]);
                t.set_primary_key(&["dummy", "one", "two"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model a {
          one Int
          two Int
          b   b[]

          @@id([one, two])
        }

        model b {
          dummy Int
          one   Int
          two   Int
          a     a   @relation(fields: [one, two], references: [one, two], onDelete: NoAction, onUpdate: NoAction)

          @@id([dummy, one, two])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Sqlite))]
async fn compound_foreign_keys_for_duplicate_one_to_many_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());

                t.add_constraint("user_unique", types::unique_constraint(&["id", "age"]));
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

          @@unique([id, age], name: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
