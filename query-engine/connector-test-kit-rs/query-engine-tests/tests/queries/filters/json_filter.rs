use query_engine_tests::*;

#[test_suite(capabilities(JsonFiltering))]
mod json_filters {

    #[connector_test(schema(json_opt))]
    async fn string_contains_does_not_error(runner: Runner) -> TestResult<()> {
        // NOTE: with string operations the results are always empty because we check for an object, not a string
        // in any case, this should not fail, it will work and return an empty result
        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findFirstTestModel(where: { json: { string_contains: "foo" }  }) { id }}"#),
            @r###"{"data":{"findFirstTestModel":null}}"###
        );

        Ok(())
    }

    #[connector_test(schema(json_opt))]
    async fn string_begins_with_does_not_error(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findFirstTestModel(where: { json: { string_starts_with: "foo" }  }) { id }}"#),
            @r###"{"data":{"findFirstTestModel":null}}"###
        );

        Ok(())
    }

    #[connector_test(schema(json_opt))]
    async fn string_ends_with_does_not_error(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findFirstTestModel(where: { json: { string_ends_with: "foo" }  }) { id }}"#),
            @r###"{"data":{"findFirstTestModel":null}}"###
        );

        Ok(())
    }
}
