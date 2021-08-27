use barrel::types;
use expect_test::expect;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Mssql))]
async fn remapping_models_in_relations_should_not_map_virtual_fields(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post With Space", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_foreign_key(&["user_id"], "User", &["id"]);

                t.add_constraint("post_user_unique", types::unique_constraint(vec!["user_id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post_With_Space {
          id      Int  @id @default(autoincrement())
          user_id Int  @unique
          User    User @relation(fields: [user_id], references: [id], onUpdate: NoAction)

          @@map("Post With Space")
        }

        model User {
          id              Int              @id @default(autoincrement())
          Post_With_Space Post_With_Space?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn remapping_models_in_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User with Space", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_foreign_key(&["user_id"], "User with Space", &["id"]);

                t.add_constraint(
                    "post_user_unique",
                    types::unique_constraint(vec!["user_id"]).unique(true),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id              Int             @id @default(autoincrement())
          user_id         Int             @unique
          User_with_Space User_with_Space @relation(fields: [user_id], references: [id], onUpdate: NoAction)
        }

        model User_with_Space {
          id   Int   @id @default(autoincrement())
          Post Post?

          @@map("User with Space")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn remapping_fields_in_compound_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age-that-is-invalid", types::integer());

                t.add_constraint(
                    "user_unique",
                    types::unique_constraint(vec!["id", "age-that-is-invalid"]),
                );
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_foreign_key(&["user_id", "user_age"], "User", &["id", "age-that-is-invalid"]);

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
          User     User @relation(fields: [user_id, user_age], references: [id, age_that_is_invalid], onUpdate: NoAction)

          @@unique([user_id, user_age], name: "post_user_unique")
        }

        model User {
          id                  Int   @id @default(autoincrement())
          age_that_is_invalid Int   @map("age-that-is-invalid")
          Post                Post?

          @@unique([id, age_that_is_invalid], name: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
