use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseOracle))]
async fn oracle_invalid_database_names_are_mapped_safely(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TYPE "123Book_1color" AS ENUM ('black', 'needs-review');

        CREATE TABLE "123Book" (
            id INTEGER PRIMARY KEY,
            "1color" "123Book_1color"
        );
        "#,
    )
    .await;

    let result = api.introspect().await?;

    for expected in [
        "model Book",
        "color Book_1color? @map(\"1color\")",
        "@@map(\"123Book\")",
        "enum Book_1color",
        "needs_review @map(\"needs-review\")",
        "@@map(\"123Book_1color\")",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    Ok(())
}
