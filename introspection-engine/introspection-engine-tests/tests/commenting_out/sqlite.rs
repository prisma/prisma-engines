use introspection_engine_tests::test_api::*;

#[test_connector(tags(Sqlite))]
async fn a_table_without_required_uniques(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE "Post" (
            id INTEGER NOT NULL,
            opt_unique INTEGER UNIQUE
        );
    "#;

    api.raw_cmd(setup).await;

    let expectation = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
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
async fn ignore_on_model_with_only_optional_id(api: &TestApi) -> TestResult {
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
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model OnlyOptionalId {
          id String? @id

          @@ignore
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
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
