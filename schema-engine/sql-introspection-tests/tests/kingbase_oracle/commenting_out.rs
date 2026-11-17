use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseOracle))]
async fn oracle_tables_without_a_required_unique_identifier_are_ignored(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE ignored_model (
            id INTEGER NOT NULL,
            optional_unique INTEGER,
            CONSTRAINT ignored_model_optional_unique_key UNIQUE (optional_unique)
        );
        "#,
    )
    .await;

    let expected = expect![[r#"
        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model ignored_model {
          id              Int
          optional_unique Int? @unique

          @@ignore
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_invalid_column_names_are_commented_out(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE quoted_fields (
            id INTEGER PRIMARY KEY,
            "1" VARCHAR2(64) NOT NULL
        );
        "#,
    )
    .await;

    let expected = expect![[r#"
        generator client {
          provider = "prisma-client"
        }

        datasource db {
          provider = "kingbase-oracle"
        }

        model quoted_fields {
          id Int @id
          /// This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 1 String @map("1") @db.VarChar2(64)
        }
    "#]];

    api.expect_datamodel(&expected).await;

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_postgres_array_columns_are_rendered_as_unsupported(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE array_values (
            id INTEGER PRIMARY KEY,
            items INTEGER[] NOT NULL
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    assert!(result.contains("model array_values"), "{result}");
    assert!(result.contains("Unsupported"), "{result}");
    assert!(!result.contains("Int[]"), "{result}");

    Ok(())
}
