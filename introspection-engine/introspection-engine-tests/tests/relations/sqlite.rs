use barrel::types;
use expect_test::expect;
use introspection_engine_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(Sqlite))]
async fn a_one_to_one_relation_referencing_non_id(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("email", types::varchar(10).unique(true).nullable(true));
            });

            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_email", types::varchar(10).unique(true).nullable(true));
                t.add_foreign_key(&["user_email"], "User", &["email"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        model Post {
          id         Int     @id @default(autoincrement())
          user_email String? @unique
          User       User?   @relation(fields: [user_email], references: [email], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id    Int     @id @default(autoincrement())
          email String? @unique
          Post  Post?
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn relations_should_avoid_name_clashes(api: &TestApi) -> TestResult {
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
          id     Int @id @default(autoincrement())
          y      Int
          y_xToy y   @relation(fields: [y], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model y {
          id     Int @id @default(autoincrement())
          x      Int
          x_xToy x[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
