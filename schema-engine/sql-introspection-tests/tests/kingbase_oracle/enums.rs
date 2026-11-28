use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseOracle))]
async fn oracle_enums_and_defaults_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TYPE mood AS ENUM ('happy', 'needs-review', '123');

        CREATE TABLE enum_defaults (
            id INTEGER PRIMARY KEY,
            current_mood mood NOT NULL DEFAULT 'happy'
        );
        "#,
    )
    .await;

    let expected = expect![[r#"
        model enum_defaults {
          id           Int  @id
          current_mood mood @default(happy)
        }

        enum mood {
          happy
          needs_review @map("needs-review")
          // 123 @map("123")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_multiple_enum_columns_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TYPE color AS ENUM ('red', 'blue');
        CREATE TYPE priority AS ENUM ('low', 'high');

        CREATE TABLE enum_columns (
            id INTEGER PRIMARY KEY,
            color_value color NOT NULL DEFAULT 'red',
            priority_value priority
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in [
        "color_value    color     @default(red)",
        "priority_value priority?",
        "enum color",
        "enum priority",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    for _ in 0..3 {
        assert_eq!(result, api.introspect_dml().await?);
    }

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_boolean_like_enum_defaults_remain_enum_values(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TYPE confirmation AS ENUM ('true', 'false', 'rumor');

        CREATE TABLE enum_boolean_defaults (
            id INTEGER PRIMARY KEY,
            confirmed confirmation NOT NULL DEFAULT 'true'
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    assert!(
        result
            .lines()
            .any(|line| line.contains("confirmed") && line.contains("confirmation") && line.contains("@default(true)")),
        "{result}"
    );
    assert!(result.contains("enum confirmation"), "{result}");

    Ok(())
}

#[test_connector(tags(KingbaseOracle))]
async fn oracle_reintrospection_preserves_mapped_enum_values_and_defaults(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TYPE binary_flag AS ENUM ('0', '1');

        CREATE TABLE enum_mapping (
            id INTEGER PRIMARY KEY,
            value binary_flag NOT NULL DEFAULT '0'
        );
        "#,
    )
    .await;

    let previous_schema = r#"
        model enum_mapping {
          id    Int         @id
          value binary_flag @default(is_false)
        }

        enum binary_flag {
          is_false @map("0")
          is_true  @map("1")
        }
    "#;

    let result = api.re_introspect_dml(previous_schema).await?;

    for expected in [
        "value binary_flag @default(is_false)",
        "is_false @map(\"0\")",
        "is_true  @map(\"1\")",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}
