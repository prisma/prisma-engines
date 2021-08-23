use query_engine_tests::*;

#[test_suite(schema(json_opt), capabilities(Json), exclude(MySQL(5.6)))]
mod json {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, json: "{}" }"#).await?;
        create_row(&runner, r#"{ id: 2, json: "{\"a\":\"b\"}" }"#).await?;
        create_row(&runner, r#"{ id: 3, json: DbNull }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json: { equals: "{}" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // Note: Added not null to keep API results compatible with Mongo
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: [{ json: { not: "{}" }}, { json: { not: DbNull }} ]}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json: { not: null }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    // defaults
    // omission

    #[connector_test]
    async fn basic_null_eq(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, json: JsonNull }"#).await?;
        create_row(&runner, r#"{ id: 2, json: "{\"a\":\"b\"}" }"#).await?;
        create_row(&runner, r#"{ id: 3, json: DbNull }"#).await?;
        create_row(&runner, r#"{ id: 4, json: "\"null\"" }"#).await?;
        create_row(&runner, r#"{ id: 5, json: "null" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json: { equals: DbNull }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json: { equals: JsonNull }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":5}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json: { equals: AnyNull }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3},{"id":5}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn no_shorthands(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"query { findManyTestModel(where: { json: "{}" }) { id }}"#,
            2009,
            "`Value types mismatch. Have: String(\"{}\"), want: Object(JsonNullableFilter)` at `Query.findManyTestModel.where.TestModelWhereInput.json`"
        );

        assert_error!(
            &runner,
            r#"query { findManyTestModel(where: { json: null }) { id }}"#,
            2012,
            "Missing a required value at `Query.findManyTestModel.where.TestModelWhereInput.json`"
        );

        Ok(())
    }

    #[connector_test]
    async fn nested_not_shorthand(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"query { findManyTestModel(where: { json: { not: { equals: "{}" }}}) { id }}"#,
            2009,
            "`Query.findManyTestModel.where.TestModelWhereInput.json.JsonNullableFilter.not`: Value types mismatch. Have: Object({\"equals\": String(\"{}\")}), want: Json"
        );

        assert_error!(
            &runner,
            r#"query { findManyTestModel(where: { json: { not: { equals: null }}}) { id }}"#,
            2009,
            "`Query.findManyTestModel.where.TestModelWhereInput.json.JsonNullableFilter.not`: Value types mismatch. Have: Object({\"equals\": Null}), want: Json"
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();

        Ok(())
    }
}
