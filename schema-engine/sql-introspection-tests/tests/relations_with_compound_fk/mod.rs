mod cockroachdb;
mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use barrel::types;
use expect_test::expect;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(exclude(Sqlite, Mysql, CockroachDb))]
async fn compound_foreign_keys_for_one_to_one_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
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
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );

                t.add_constraint(
                    "post_user_unique",
                    types::unique_constraint(vec!["user_id", "user_age"]),
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

          @@unique([user_id, user_age], map: "post_user_unique")
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

#[test_connector(exclude(Mysql, Mssql, CockroachDb))]
async fn compound_foreign_keys_for_required_one_to_many_relations(api: &mut TestApi) -> TestResult {
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

                t.add_constraint(
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
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

#[test_connector(exclude(Sqlite, Mysql, Mssql, CockroachDb))]
async fn compound_foreign_keys_for_required_self_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer());
                t.add_column("partner_age", types::integer());

                t.add_constraint(
                    "Person_pid_page_fkey",
                    types::foreign_constraint(&["partner_id", "partner_age"], "Person", &["id", "age"], None, None),
                );
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
          Person       Person   @relation("PersonToPerson", fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction, map: "Person_pid_page_fkey")
          other_Person Person[] @relation("PersonToPerson")

          @@unique([id, age], map: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Sqlite, CockroachDb))]
async fn compound_foreign_keys_for_self_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer().nullable(true));
                t.add_column("partner_age", types::integer().nullable(true));

                t.add_constraint(
                    "Person_fkey",
                    types::foreign_constraint(&["partner_id", "partner_age"], "Person", &["id", "age"], None, None),
                );
                t.add_constraint("post_user_unique", types::unique_constraint(vec!["id", "age"]));
                t.add_constraint("Person_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Person {
          id           Int      @id @default(autoincrement())
          age          Int
          partner_id   Int?
          partner_age  Int?
          Person       Person?  @relation("PersonToPerson", fields: [partner_id, partner_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction, map: "Person_fkey")
          other_Person Person[] @relation("PersonToPerson")

          @@unique([id, age], map: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(Mysql, Mssql, CockroachDb))]
async fn repro_matt_references_on_wrong_side(api: &mut TestApi) -> TestResult {
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
                t.add_constraint(
                    "b_one_two_fkey",
                    types::foreign_constraint(&["one", "two"], "a", &["one", "two"], None, None),
                );
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

#[test_connector(exclude(Mysql, Mssql, CockroachDb))]
async fn a_compound_fk_pk_with_overlapping_primary_key(api: &mut TestApi) -> TestResult {
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

                t.add_constraint(
                    "b_one_two_fkey",
                    types::foreign_constraint(&["one", "two"], "a", &["one", "two"], None, None),
                );
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

#[test_connector(exclude(Mysql, Sqlite, CockroachDb))]
async fn compound_foreign_keys_for_duplicate_one_to_many_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("age", types::integer());

                t.add_constraint("user_unique", types::unique_constraint(["id", "age"]));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));
                t.add_column("other_user_id", types::integer().nullable(true));
                t.add_column("other_user_age", types::integer().nullable(true));

                t.add_constraint(
                    "Post_fk1",
                    types::foreign_constraint(&["user_id", "user_age"], "User", &["id", "age"], None, None),
                );
                t.add_constraint(
                    "Post_fk2",
                    types::foreign_constraint(&["other_user_id", "other_user_age"], "User", &["id", "age"], None, None),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
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
          User_Post_user_id_user_ageToUser             User? @relation("Post_user_id_user_ageToUser", fields: [user_id, user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction, map: "Post_fk1")
          User_Post_other_user_id_other_user_ageToUser User? @relation("Post_other_user_id_other_user_ageToUser", fields: [other_user_id, other_user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction, map: "Post_fk2")
        }

        model User {
          id                                           Int    @id @default(autoincrement())
          age                                          Int
          Post_Post_user_id_user_ageToUser             Post[] @relation("Post_user_id_user_ageToUser")
          Post_Post_other_user_id_other_user_ageToUser Post[] @relation("Post_other_user_id_other_user_ageToUser")

          @@unique([id, age], map: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
