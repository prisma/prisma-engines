use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseOracle), preview_features("views"))]
async fn oracle_views_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE source_rows (
            id INTEGER PRIMARY KEY,
            label VARCHAR2(64) NOT NULL,
            optional_label VARCHAR2(64)
        );

        CREATE VIEW source_row_view AS
            SELECT id, label, optional_label FROM source_rows;
        "#,
    )
    .await;

    let expected = expect![[r#"
        generator client {
          provider        = "prisma-client"
          previewFeatures = ["views"]
        }

        datasource db {
          provider = "kingbase-oracle"
        }

        model source_rows {
          id             Int     @id
          label          String  @db.VarChar2(64)
          optional_label String? @db.VarChar2(64)
        }

        view source_row_view {
          id             Int?
          label          String? @db.VarChar2(64)
          optional_label String? @db.VarChar2(64)
        }
    "#]];

    api.expect_datamodel(&expected).await;
    api.expect_no_warnings().await;

    Ok(())
}

#[test_connector(tags(KingbaseOracle), preview_features("views"))]
async fn oracle_join_and_view_to_view_definitions_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE view_users (
            id INTEGER PRIMARY KEY,
            name VARCHAR2(64) NOT NULL
        );

        CREATE TABLE view_profiles (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL,
            biography CLOB,
            CONSTRAINT view_profiles_user_fk FOREIGN KEY (user_id) REFERENCES view_users (id)
        );

        CREATE VIEW user_profiles AS
            SELECT u.id, u.name, p.biography
            FROM view_users u
            INNER JOIN view_profiles p ON p.user_id = u.id;

        CREATE VIEW user_profile_names AS
            SELECT id, name FROM user_profiles;
        "#,
    )
    .await;

    let result = api.introspect().await?;

    for expected in [
        "model view_users",
        "model view_profiles",
        "view user_profiles",
        "view user_profile_names",
        "biography String? @db.Clob",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    let definitions = api.introspect_views().await?.unwrap_or_default();
    let user_profiles = definitions
        .iter()
        .find(|view| view.name == "user_profiles")
        .expect("user_profiles view definition was not introspected");
    assert!(user_profiles.definition.to_ascii_lowercase().contains("select"));
    assert!(definitions.iter().any(|view| view.name == "user_profile_names"));

    Ok(())
}

#[test_connector(tags(KingbaseOracle), preview_features("views"))]
async fn oracle_reintrospection_keeps_view_relations(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE VIEW a AS SELECT 1 AS id;
        CREATE VIEW b AS SELECT 2 AS id, 1 AS a_id;
        "#,
    )
    .await;

    let previous_schema = r#"
        view a {
          id Int @unique
          b  b[]
        }

        view b {
          id   Int  @unique
          a_id Int?
          a    a?   @relation(fields: [a_id], references: [id])
        }
    "#;

    let result = api.re_introspect_dml(previous_schema).await?;

    for expected in [
        "view a",
        "b  b[]",
        "view b",
        "a    a?   @relation(fields: [a_id], references: [id])",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}

#[test_connector(tags(KingbaseOracle), preview_features("views"))]
async fn oracle_invalid_view_column_names_are_commented_out(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE source_rows (
            id INTEGER PRIMARY KEY
        );

        CREATE VIEW invalid_view_column AS
            SELECT id AS "1" FROM source_rows;
        "#,
    )
    .await;

    let result = api.introspect().await?;

    assert!(result.contains("view invalid_view_column"), "{result}");
    assert!(result.contains("// 1 Int? @map(\"1\")"), "{result}");

    let warnings = api.introspection_warnings().await?;
    assert!(warnings.contains("invalid_view_column"), "{warnings}");

    Ok(())
}

#[test_connector(tags(KingbaseOracle), preview_features("views"))]
async fn oracle_reintrospection_keeps_view_documentation(api: &mut TestApi) -> TestResult {
    api.raw_cmd("CREATE VIEW documented_view AS SELECT 1 AS id;").await;

    let previous_schema = r#"
        /// User-provided view documentation.
        view documented_view {
          /// User-provided field documentation.
          id Int
        }
    "#;

    let result = api.re_introspect_dml(previous_schema).await?;

    assert!(result.contains("/// User-provided view documentation."), "{result}");
    assert!(result.contains("/// User-provided field documentation."), "{result}");

    Ok(())
}

#[test_connector(tags(KingbaseOracle), preview_features("views"))]
async fn oracle_views_preserve_enum_types_and_table_defaults(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TYPE view_status AS ENUM ('draft', 'published');

        CREATE TABLE view_source (
            id INTEGER PRIMARY KEY,
            status view_status NOT NULL DEFAULT 'draft'
        );

        CREATE VIEW status_view AS
            SELECT id, status FROM view_source;
        "#,
    )
    .await;

    let result = api.introspect().await?;

    for expected in [
        "status view_status @default(draft)",
        "view status_view",
        "status view_status?",
        "enum view_status",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}
