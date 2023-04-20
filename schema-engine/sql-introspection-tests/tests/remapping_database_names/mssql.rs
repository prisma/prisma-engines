use barrel::types;
use expect_test::expect;
use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Mssql))]
async fn remapping_models_in_relations_should_not_map_virtual_fields(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]))
            });

            migration.create_table("Post With Space", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer());

                t.add_constraint(
                    "user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
                t.add_constraint("post_user_unique", types::unique_constraint(vec!["user_id"]));
                t.add_constraint("Post With Space_pkey", types::primary_constraint(vec!["id"]))
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post_With_Space {
          id      Int  @id @default(autoincrement())
          user_id Int  @unique(map: "post_user_unique")
          User    User @relation(fields: [user_id], references: [id], onUpdate: NoAction, map: "user_id_fkey")

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
async fn remapping_models_in_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User with Space", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_constraint("User with Space_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer());

                t.add_constraint(
                    "user_id_fkey",
                    types::foreign_constraint(&["user_id"], "User with Space", &["id"], None, None),
                );
                t.add_constraint(
                    "post_user_unique",
                    types::unique_constraint(vec!["user_id"]).unique(true),
                );
                t.add_constraint("Post_pkey", types::primary_constraint(["id"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id              Int             @id @default(autoincrement())
          user_id         Int             @unique(map: "post_user_unique")
          User_with_Space User_with_Space @relation(fields: [user_id], references: [id], onUpdate: NoAction, map: "user_id_fkey")
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
async fn remapping_fields_in_compound_relations(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("age-that-is-invalid", types::integer());

                t.add_constraint(
                    "user_unique",
                    types::unique_constraint(vec!["id", "age-that-is-invalid"]),
                );
                t.add_constraint("User_pkey", types::primary_constraint(["id"]));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::integer().increments(true));
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_constraint(
                    "Post_fkey",
                    types::foreign_constraint(
                        &["user_id", "user_age"],
                        "User",
                        &["id", "age-that-is-invalid"],
                        None,
                        None,
                    ),
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
          User     User @relation(fields: [user_id, user_age], references: [id, age_that_is_invalid], onUpdate: NoAction, map: "Post_fkey")

          @@unique([user_id, user_age], map: "post_user_unique")
        }

        model User {
          id                  Int   @id @default(autoincrement())
          age_that_is_invalid Int   @map("age-that-is-invalid")
          Post                Post?

          @@unique([id, age_that_is_invalid], map: "user_unique")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
