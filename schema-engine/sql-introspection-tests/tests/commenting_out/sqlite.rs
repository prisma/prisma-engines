use barrel::types;
use sql_introspection_tests::test_api::*;

#[test_connector(tags(Sqlite))]
async fn a_table_without_required_uniques(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE "Post" (
            id INTEGER NOT NULL,
            opt_unique INTEGER UNIQUE
        );
    "#;

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model Post {
          id         Int
          opt_unique Int? @unique(map: "sqlite_autoindex_Post_1")

          @@ignore
        }
    "#]];

    let introspected = api.introspect_dml().await?;

    expectation.assert_eq(&introspected);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn ignore_on_model_with_only_optional_id(api: &mut TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("ValidId", |t| {
                t.inject_custom("id Text Primary Key Not Null");
            });

            migration.create_table("OnlyOptionalId", |t| {
                t.inject_custom("id Text Primary Key");
            });

            migration.create_table("OptionalIdAndOptionalUnique", |t| {
                t.inject_custom("id Text Primary Key");
                t.add_column("unique", barrel::types::integer().unique(true).nullable(true));
            });
        })
        .await?;

    let expectation = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model OnlyOptionalId {
          id String? @id

          @@ignore
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model OptionalIdAndOptionalUnique {
          id     String? @id
          unique Int?    @unique(map: "sqlite_autoindex_OptionalIdAndOptionalUnique_2")

          @@ignore
        }

        model ValidId {
          id String @id
        }
    "#]];

    let introspected = api.introspect_dml().await?;

    expectation.assert_eq(&introspected);

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn field_with_empty_name(api: &mut TestApi) -> TestResult {
    api.raw_cmd(r#"CREATE TABLE "A"(" " INTEGER PRIMARY KEY)"#).await;

    let expectation = expect![[r#"
        model A {
          /// This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          //   Int @id @default(autoincrement()) @map(" ")
        }
    "#]];

    let introspected = api.introspect_dml().await?;
    expectation.assert_eq(&introspected);

    Ok(())
}

#[test_connector(tags(Sqlite))]
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
          provider = "sqlite"
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
