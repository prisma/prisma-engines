use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseOracle))]
async fn oracle_custom_foreign_key_names_are_rendered_with_map(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE parent (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE child (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER NOT NULL,
            CONSTRAINT custom_parent_reference
                FOREIGN KEY (parent_id) REFERENCES parent (id)
        );
        "#,
    )
    .await;

    let expected = expect![[r#"
        model child {
          id        Int    @id
          parent_id Int
          parent    parent @relation(fields: [parent_id], references: [id], onDelete: NoAction, onUpdate: NoAction, map: "custom_parent_reference")
        }

        model parent {
          id    Int     @id
          child child[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_default_foreign_key_names_are_not_rendered_with_map(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE parent (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE child (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER NOT NULL,
            FOREIGN KEY (parent_id) REFERENCES parent (id)
        );
        "#,
    )
    .await;

    let expected = expect![[r#"
        model child {
          id        Int    @id
          parent_id Int
          parent    parent @relation(fields: [parent_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model parent {
          id    Int     @id
          child child[]
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_named_primary_unique_and_index_constraints_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE named_constraints (
            id INTEGER NOT NULL,
            code VARCHAR2(32) NOT NULL,
            label VARCHAR2(32) NOT NULL,
            CONSTRAINT named_constraints_pk PRIMARY KEY (id),
            CONSTRAINT custom_code_unique UNIQUE (code)
        );

        CREATE INDEX custom_label_index ON named_constraints (label);
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in [
        "@id(map: \"named_constraints_pk\")",
        "@unique(map: \"custom_code_unique\")",
        "@@index([label], map: \"custom_label_index\")",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}
