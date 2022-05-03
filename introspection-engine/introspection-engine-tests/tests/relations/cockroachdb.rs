use barrel::types;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
async fn a_one_to_one_relation_referencing_non_id(api: &TestApi) -> TestResult {
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
          user_email String? @unique @db.String(10)
          User       User?   @relation(fields: [user_email], references: [email], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id    Int     @id @default(autoincrement())
          email String? @unique @db.String(10)
          Post  Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
