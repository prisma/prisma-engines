use query_engine_tests::*;

#[test_suite(capabilities(Json), exclude(MySQL(5.6)))]
mod json {
    use query_engine_tests::{assert_error, jNull, run_query, ConnectorCapability};
    use query_tests_setup::Runner;

    #[connector_test(schema(json_opt))]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, json: "{}" }"#).await?;
        create_row(&runner, r#"{ id: 2, json: "{\"a\":\"b\"}" }"#).await?;
        create_row(&runner, r#"{ id: 3, json: DbNull }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { json: { equals: "{}" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        let caps = &runner.connector().capabilities();

        // Note: Added not null to keep API results compatible with Mongo
        insta::assert_snapshot!(
          run_query!(&runner, jNull!(caps, r#"query { findManyTestModel(where: { AND: [{ json: { not: "{}" }}, { json: { not: DbNull }} ]}) { id }}"#)),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, jNull!(caps, r#"query { findManyTestModel(where: { json: { not: DbNull }}) { id }}"#)),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(json_opt))]
    async fn basic_null_eq(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, json: JsonNull }"#).await?;
        create_row(&runner, r#"{ id: 2, json: "{\"a\":\"b\"}" }"#).await?;
        create_row(&runner, r#"{ id: 3, json: DbNull }"#).await?;
        create_row(&runner, r#"{ id: 4, json: "\"null\"" }"#).await?;
        create_row(&runner, r#"{ id: 5, json: "null" }"#).await?;

        if runner
            .connector()
            .capabilities()
            .contains(&ConnectorCapability::AdvancedJsonNullability)
        {
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
        } else {
            insta::assert_snapshot!(
              run_query!(&runner, r#"query { findManyTestModel(where: { json: { equals: null }}) { id }}"#),
              @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3},{"id":5}]}}"###
            );

            insta::assert_snapshot!(
              run_query!(&runner, r#"query { findManyTestModel(where: { json: { equals: "null" }}) { id }}"#),
              @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3},{"id":5}]}}"###
            );
        }

        Ok(())
    }

    #[connector_test(schema(json_opt))]
    async fn basic_not_null_eq(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, json: JsonNull }"#).await?;
        create_row(&runner, r#"{ id: 2, json: "{\"a\":\"b\"}" }"#).await?;
        create_row(&runner, r#"{ id: 3, json: DbNull }"#).await?;
        create_row(&runner, r#"{ id: 4, json: "\"null\"" }"#).await?;
        create_row(&runner, r#"{ id: 5, json: "null" }"#).await?;

        if runner
            .connector()
            .capabilities()
            .contains(&ConnectorCapability::AdvancedJsonNullability)
        {
            insta::assert_snapshot!(
                run_query!(&runner, r#"query { findManyTestModel(where: { NOT: [{ json: { equals: DbNull } }]}) { id }}"#),
                @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
            );

            // DB NULLs are not included, in line with our other filters.
            insta::assert_snapshot!(
                run_query!(&runner, r#"query { findManyTestModel(where: { NOT: [{ json: { equals: JsonNull } }]}) { id }}"#),
                @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
            );

            insta::assert_snapshot!(
                run_query!(&runner, r#"query { findManyTestModel(where: { NOT: [{ json: { equals: AnyNull } }]}) { id }}"#),
                @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
            );
        } else {
            insta::assert_snapshot!(
                run_query!(&runner, r#"query { findManyTestModel(where: { NOT: [{ json: { equals: null } }]}) { id }}"#),
                @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
            );

            insta::assert_snapshot!(
                run_query!(&runner, r#"query { findManyTestModel(where: { NOT: [{ json: { equals: "null" } }]}) { id }}"#),
                @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
            );
        }

        Ok(())
    }

    #[connector_test(schema(json))]
    async fn req_json_null_filters(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, json: JsonNull }"#).await?;
        create_row(&runner, r#"{ id: 2, json: "{\"a\":\"b\"}" }"#).await?;

        create_row(&runner, r#"{ id: 4, json: "\"null\"" }"#).await?;
        create_row(&runner, r#"{ id: 5, json: "null" }"#).await?;

        if runner
            .connector()
            .capabilities()
            .contains(&ConnectorCapability::AdvancedJsonNullability)
        {
            runner
                .query("mutation { createOneTestModel(data: { id: 1, json: DbNull}) { id }}")
                .await?
                .assert_failure(
                    2009,
                    Some("Enum value 'DbNull' is invalid for enum type JsonNullValueInput".to_owned()),
                );

            insta::assert_snapshot!(
              run_query!(&runner, r#"query { findManyTestModel(where: { NOT: [{ json: { equals: DbNull } }]}) { id }}"#),
              @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
            );

            // DB NULLs are not included, in line with our other filters.
            insta::assert_snapshot!(
              run_query!(&runner, r#"query { findManyTestModel(where: { NOT: [{ json: { equals: JsonNull } }]}) { id }}"#),
              @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
            );

            insta::assert_snapshot!(
              run_query!(&runner, r#"query { findManyTestModel(where: { NOT: [{ json: { equals: AnyNull } }]}) { id }}"#),
              @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
            );
        } else {
            insta::assert_snapshot!(
              run_query!(&runner, r#"query { findManyTestModel(where: { NOT: [{ json: { equals: "null" } }]}) { id }}"#),
              @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
            );
        }

        Ok(())
    }

    #[connector_test(schema(json_default))]
    async fn basic_null_eq_defaults(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, json: "{\"a\":\"b\"}" }"#).await?;
        create_row(&runner, r#"{ id: 2 }"#).await?;
        create_row(&runner, r#"{ id: 3, json: JsonNull }"#).await?;

        let caps = &runner.connector().capabilities();

        insta::assert_snapshot!(
          run_query!(&runner, jNull!(caps, r#"query { findManyTestModel(where: { json: { equals: JsonNull }}) { id }}"#)),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
        );

        if runner
            .connector()
            .capabilities()
            .contains(&ConnectorCapability::AdvancedJsonNullability)
        {
            // Should work, but not useful with req. fields.
            insta::assert_snapshot!(
              run_query!(&runner, jNull!(caps, r#"query { findManyTestModel(where: { json: { equals: AnyNull }}) { id }}"#)),
              @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
            );
        }

        Ok(())
    }

    #[connector_test(schema(json_opt))]
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

    #[connector_test(schema(json_opt))]
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
        let caps = &runner.connector().capabilities();

        runner
            .query(jNull!(
                caps,
                format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data)
            ))
            .await?
            .assert_success();

        Ok(())
    }
}
