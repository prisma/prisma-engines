mod cockroachdb;
mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use barrel::types;
use sql_introspection_tests::test_api::*;

#[test_connector(exclude(Mssql, Mysql, CockroachDb))]
async fn a_table_without_uniques_should_ignore(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("user_id", types::integer().nullable(false));
                t.add_index("Post_user_id_idx", types::index(["user_id"]));
                t.add_foreign_key(&["user_id"], "User", &["id"]);
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Post {
          id      Int
          user_id Int
          User    User @relation(fields: [user_id], references: [id], onDelete: NoAction, onUpdate: NoAction)

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

#[test_connector(exclude(Sqlite, Mysql))]
async fn a_table_without_required_uniques(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("opt_unique", types::integer().nullable(true));

                t.add_constraint("sqlite_autoindex_Post_1", types::unique_constraint(vec!["opt_unique"]));
            });
        })
        .await?;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Post {
          id         Int
          opt_unique Int? @unique(map: "sqlite_autoindex_Post_1")

          @@ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(exclude(CockroachDb))] // there is no such thing on cockroach, you will get the rowid column
async fn a_table_without_fully_required_compound_unique(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::integer());
                t.add_column("opt_unique", types::integer().nullable(true));
                t.add_column("req_unique", types::integer().nullable(false));

                t.add_constraint(
                    "sqlite_autoindex_Post_1",
                    types::unique_constraint(vec!["opt_unique", "req_unique"]),
                );
            });
        })
        .await?;

    let dm = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Post {
          id         Int
          opt_unique Int?
          req_unique Int

          @@unique([opt_unique, req_unique], map: "sqlite_autoindex_Post_1")
          @@ignore
        }
    "#]];

    let result = api.introspect_dml().await?;
    dm.assert_eq(&result);

    Ok(())
}

#[test_connector(exclude(CockroachDb, Mysql, Mssql, Sqlite))]
async fn remapping_field_names_to_empty(api: &mut TestApi) -> TestResult {
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
          provider = "prisma-client"
        }

        datasource db {
          provider = "postgresql"
          url      = "dummy-url"
        }

        model User {
          /// This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 1 String @map("1")
          last Int @id @default(autoincrement())
        }
    "#]];

    api.expect_datamodel(&dm).await;

    Ok(())
}
