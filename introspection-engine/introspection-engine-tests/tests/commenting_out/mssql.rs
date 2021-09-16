use barrel::types;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Mssql))]
async fn a_table_without_uniques_should_ignore(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::integer());
                t.add_constraint("User_pkey", types::primary_constraint(vec!["id"]));
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_idx", types::index(&["user_id"]));
                t.add_constraint(
                    "thefk",
                    types::foreign_constraint(&["user_id"], "User", &["id"], None, None),
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Post {
          id      Int
          user_id Int
          User    User @relation(fields: [user_id], references: [id], onUpdate: NoAction, map: "thefk")

          @@index([user_id])
          @@ignore
        }

        model User {
          id   Int    @id
          Post Post[] @ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
