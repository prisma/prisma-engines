use barrel::types;
use expect_test::expect;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Mssql))]
async fn compound_foreign_keys_for_required_one_to_one_relations(api: &TestApi) -> TestResult {
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
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_constraint(
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(["user_id", "user_age"], "User", ["id", "age"], None, None),
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
          id       Int  @id @default(autoincrement())
          user_id  Int
          user_age Int
          User     User @relation(fields: [user_id, user_age], references: [id, age], onUpdate: NoAction)

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

#[test_connector(tags(Mssql))]
async fn a_compound_fk_pk_with_overlapping_primary_key(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));

                t.add_constraint("a_pkey", types::primary_constraint(["one", "two"]));
            });
            migration.create_table("b", |t| {
                t.add_column("dummy", types::integer().nullable(false));
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));

                t.add_constraint(
                    "b_one_two_fkey",
                    types::foreign_constraint(["one", "two"], "a", ["one", "two"], None, None),
                );
                t.add_constraint("b_pkey", types::primary_constraint(["dummy", "one", "two"]));
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
          a     a   @relation(fields: [one, two], references: [one, two], onUpdate: NoAction)

          @@id([dummy, one, two])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn compound_foreign_keys_for_one_to_many_relations_with_mixed_requiredness(api: &TestApi) -> TestResult {
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
                t.add_column("user_id", types::integer().nullable(false));
                t.add_column("user_age", types::integer().nullable(true));

                t.add_constraint(
                    "Post_fkey",
                    types::foreign_constraint(["user_id", "user_age"], "User", ["id", "age"], None, None),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id       Int   @id @default(autoincrement())
          user_id  Int
          user_age Int?
          User     User? @relation(fields: [user_id, user_age], references: [id, age], onUpdate: NoAction, map: "Post_fkey")
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

#[test_connector(tags(Mssql))]
async fn compound_foreign_keys_for_required_one_to_many_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("age", types::integer());

                t.add_index("user_unique", types::index(vec!["id", "age"]).unique(true));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_constraint(
                    "Post_user_id_user_age_fkey",
                    types::foreign_constraint(["user_id", "user_age"], "User", ["id", "age"], None, None),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id       Int  @id @default(autoincrement())
          user_id  Int
          user_age Int
          User     User @relation(fields: [user_id, user_age], references: [id, age], onUpdate: NoAction)
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

#[test_connector(tags(Mssql))]
async fn repro_matt_references_on_wrong_side(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("a", |t| {
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));
                t.add_constraint("a_pkey", types::primary_constraint(["one", "two"]));
            });

            migration.create_table("b", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("one", types::integer().nullable(false));
                t.add_column("two", types::integer().nullable(false));
                t.add_constraint(
                    "b_one_two_fkey",
                    types::foreign_constraint(["one", "two"], "a", ["one", "two"], None, None),
                );
                t.add_constraint("b_pkey", types::primary_constraint(["id"]));
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
          a   a   @relation(fields: [one, two], references: [one, two], onUpdate: NoAction)
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn compound_foreign_keys_for_one_to_many_relations_with_non_unique_index(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("age", types::integer());

                t.add_constraint("post_user_unique", types::unique_constraint(vec!["id", "age"]));
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_constraint(
                    "Post_user_id_user_face_fkey",
                    types::foreign_constraint(["user_id", "user_age"], "User", ["id", "age"], None, None),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id       Int  @id @default(autoincrement())
          user_id  Int
          user_age Int
          User     User @relation(fields: [user_id, user_age], references: [id, age], onUpdate: NoAction, map: "Post_user_id_user_face_fkey")
        }

        model User {
          id   Int    @id @default(autoincrement())
          age  Int
          Post Post[]

          @@unique([id, age], map: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn compound_foreign_keys_for_required_self_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer());
                t.add_column("partner_age", types::integer());

                t.add_constraint(
                    "person_fkey",
                    types::foreign_constraint(["partner_id", "partner_age"], "Person", ["id", "age"], None, None),
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
          partner_id   Int
          partner_age  Int
          Person       Person   @relation("PersonToPerson", fields: [partner_id, partner_age], references: [id, age], onUpdate: NoAction, map: "person_fkey")
          other_Person Person[] @relation("PersonToPerson")

          @@unique([id, age], map: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn compound_foreign_keys_with_defaults(api: &TestApi) -> TestResult {
    let setup = format!(
        r#"
        CREATE TABLE [{schema}].[Person] (
            id INTEGER IDENTITY NOT NULL,
            age INTEGER NOT NULL,
            partner_id INTEGER CONSTRAINT [partner_id_default] DEFAULT 0 NOT NULL,
            partner_age INTEGER CONSTRAINT [partner_age_default] DEFAULT 0 NOT NULL,

            CONSTRAINT [partner_id_age_fkey] FOREIGN KEY (partner_id, partner_age) REFERENCES [{schema}].[Person](id, age),
            CONSTRAINT [post_user_unique] UNIQUE (id, age),
            CONSTRAINT [Person_pkey] PRIMARY KEY (id)
        );
        "#,
        schema = api.schema_name(),
    );

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model Person {
          id           Int      @id @default(autoincrement())
          age          Int
          partner_id   Int      @default(0, map: "partner_id_default")
          partner_age  Int      @default(0, map: "partner_age_default")
          Person       Person   @relation("PersonToPerson", fields: [partner_id, partner_age], references: [id, age], onUpdate: NoAction, map: "partner_id_age_fkey")
          other_Person Person[] @relation("PersonToPerson")

          @@unique([id, age], map: "post_user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn compound_foreign_keys_for_one_to_many_relations(api: &TestApi) -> TestResult {
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
                    types::foreign_constraint(["user_id", "user_age"], "User", ["id", "age"], None, None),
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
          User     User? @relation(fields: [user_id, user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction, map: "Post_fk")
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
