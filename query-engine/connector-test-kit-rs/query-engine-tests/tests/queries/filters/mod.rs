use query_engine_tests::*;

pub mod bigint_filter;
pub mod bytes_filter;
pub mod decimal_filter;
pub mod extended_relation_filters;
pub mod filter_regression;
pub mod filter_unwrap;
pub mod filters;
pub mod insensitive_filters;
pub mod json;
pub mod json_path;
pub mod list_filters;
pub mod many_relation;
pub mod one2one_regression;
pub mod one_relation;
pub mod ported_filters;
pub mod relation_null;
pub mod search_filter;
pub mod self_relation;
pub mod self_relation_regression;
pub mod where_unique;

/// Creates test data used by filter tests using the `common_nullable_types` schema.
/// ```text
/// id | string | bInt | float | decimal | bytes      | bool | dt
/// 1  | null   | 5    | null  | 5.5     | "dGVzdA==" | null | null
/// 2  | null   | 1    | null  | 1       | "dA=="     | null | null
/// 3  | null   | null | null  | null    | null       | null | null
/// ```
async fn common_test_data(runner: &Runner) -> TestResult<()> {
    runner
        .query(indoc! { r#"
            mutation { createOneTestModel(data: {
                id: 1,
                bInt: 5,
                decimal: "5.5",
                bytes: "dGVzdA==",
            }) { id }}"# })
        .await?
        .assert_success();

    runner
        .query(indoc! { r#"
            mutation { createOneTestModel(data: {
                id: 2,
                bInt: 1,
                decimal: "1",
                bytes: "dA==",
            }) { id }}"# })
        .await?
        .assert_success();

    runner
        .query(indoc! { r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"# })
        .await?
        .assert_success();

    Ok(())
}
