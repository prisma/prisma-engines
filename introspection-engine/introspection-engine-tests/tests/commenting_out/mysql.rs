use barrel::types;
use introspection_engine_tests::{test_api::*, TestResult};

#[test_connector(tags(Mysql))]
async fn a_table_without_required_uniques(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("opt_unique", types::integer().unique(true).nullable(true));
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Post {
          id         Int
          opt_unique Int? @unique(map: "opt_unique")

          @@ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn a_table_without_uniques_should_ignore(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_idx", types::index(&["user_id"]));

                t.inject_custom(
                    "CONSTRAINT user_id FOREIGN KEY (user_id) REFERENCES `User`(id) ON DELETE RESTRICT ON UPDATE CASCADE",
                );
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model Post {
          id      Int
          user_id Int
          User    User @relation(fields: [user_id], references: [id], map: "user_id")

          @@index([user_id])
          @@ignore
        }

        model User {
          id   Int    @id @default(autoincrement())
          Post Post[] @ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn remapping_field_names_to_empty_mysql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("1", types::text());
                t.add_column("last", types::integer().increments(true));

                t.add_constraint("User_pkey", types::primary_constraint(vec!["last"]));
            });
        })
        .await?;

    let dm = expect![[r#"
        generator client {
          provider = "prisma-client-js"
        }

        datasource db {
          provider = "mysql"
          url      = "env(TEST_DATABASE_URL)"
        }

        model User {
          /// This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 1 String @map("1") @db.Text
          last Int @id @default(autoincrement())
        }
    "#]];

    api.expect_datamodel(&dm).await;

    Ok(())
}
