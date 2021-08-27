use barrel::types;
use expect_test::expect;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Sqlite))]
async fn remapping_models_in_compound_relations(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(move |migration| {
            migration.create_table("User with Space", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());

                t.add_constraint(
                    "sqlite_autoindex_User with Space_1",
                    types::unique_constraint(vec!["id", "age"]),
                );
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());

                t.add_foreign_key(&["user_id", "user_age"], "User with Space", &["id", "age"]);

                t.add_constraint(
                    "sqlite_autoindex_Post_1",
                    types::unique_constraint(vec!["user_id", "user_age"]).unique(true),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id              Int             @id @default(autoincrement())
          user_id         Int
          user_age        Int
          User_with_Space User_with_Space @relation(fields: [user_id, user_age], references: [id, age], onDelete: NoAction, onUpdate: NoAction)

          @@unique([user_id, user_age], name: "sqlite_autoindex_Post_1")
        }

        model User_with_Space {
          id   Int   @id @default(autoincrement())
          age  Int
          Post Post?

          @@unique([id, age], name: "sqlite_autoindex_User with Space_1")
          @@map("User with Space")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
