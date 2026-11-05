use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseOracle), namespaces("first", "second"))]
async fn oracle_multiple_schemas_with_cross_schema_relation_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE SCHEMA first;
        CREATE SCHEMA second;

        CREATE TABLE first.account (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE second.account (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE second.profile (
            id INTEGER PRIMARY KEY,
            account_id INTEGER NOT NULL,
            CONSTRAINT profile_account_fk FOREIGN KEY (account_id) REFERENCES first.account (id)
        );
        "#,
    )
    .await;

    let result = api.introspect().await?;

    for expected in [
        "provider = \"kingbase-oracle\"",
        "schemas  = [\"first\", \"second\"]",
        "model first_account",
        "model second_account",
        "model profile",
        "@@schema(\"first\")",
        "@@schema(\"second\")",
        "fields: [account_id], references: [id]",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}

#[test_connector(tags(KingbaseOracle), namespaces("first", "second"))]
async fn oracle_duplicate_enum_names_in_multiple_schemas_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE SCHEMA first;
        CREATE SCHEMA second;

        CREATE TYPE first.mood AS ENUM ('happy');
        CREATE TYPE second.mood AS ENUM ('sad');

        CREATE TABLE first.person (
            id INTEGER PRIMARY KEY,
            current_mood first.mood NOT NULL
        );

        CREATE TABLE second.person (
            id INTEGER PRIMARY KEY,
            current_mood second.mood NOT NULL
        );
        "#,
    )
    .await;

    let result = api.introspect().await?;

    for expected in [
        "model first_person",
        "model second_person",
        "enum first_mood",
        "enum second_mood",
        "@@schema(\"first\")",
        "@@schema(\"second\")",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}

#[test_connector(tags(KingbaseOracle), namespaces("first", "second"))]
async fn oracle_multiple_schemas_preserve_mapped_models_on_reintrospection(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE SCHEMA first;
        CREATE SCHEMA second;

        CREATE TABLE first.item (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE second.item (
            id INTEGER PRIMARY KEY
        );
        "#,
    )
    .await;

    let previous_schema = r#"
        model FirstItem {
          item_id Int @id @map("id")

          @@map("item")
          @@schema("first")
        }

        model SecondItem {
          item_id Int @id @map("id")

          @@map("item")
          @@schema("second")
        }
    "#;

    let result = api.re_introspect(previous_schema).await?;

    for expected in [
        "model FirstItem",
        "model SecondItem",
        "item_id Int @id @map(\"id\")",
        "@@map(\"item\")",
        "@@schema(\"first\")",
        "@@schema(\"second\")",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}
