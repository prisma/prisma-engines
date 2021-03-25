use query_engine_tests::prelude::*;

// The mod name dictates the db name. If the name is `some_spec`
// then, for example, the MySQL db should be (similar to) `some_spec` as well.
// #[schema(schema_handler)] // Schema for all contained tests. Allows us to cache runners maybe.
#[test_suite(schema(schemas::some_common_schema), only(Postgres(11)))]
mod some_spec {
    // These imports are required if no `#[test_suite(...)]` is used
    // use super::*;
    // use query_tests_setup::*;
    // use std::convert::TryFrom;

    // Handler that returns a schema template to use for rendering.
    // Template rendering can be bypassed by simply not using the template strings.
    // Common schema handlers to use should be in a central place.
    fn schema_handler() -> String {
        "model A {
            #id(id, Int, @id)
            field String?
        }"
        .to_owned()
    }

    #[connector_test(suite = "named_suite", schema(schema_handler), only(Sqlite))]
    async fn ideal_api_test(runner: &Runner) -> TestResult<()> {
        let result = runner
            .query(indoc! {r#"
                    mutation {
                        createOneA(data: { id: 1, field: "1"}) { id }
                    }
                "#,
            })
            .await?;

        assert_eq!(result.to_string(), r#"{"data":{"createOneA":{"id":1}}}"#);
        Ok(())
    }

    #[connector_test]
    async fn other_test(runner: &Runner) -> TestResult<()> {
        let result = runner
            .query(indoc! {r#"
                mutation {
                    createOneC(data: { id: 1, field: "1"}) { id }
                }
            "#,
            })
            .await?;

        assert_eq!(result.to_string(), r#"{"data":{"createOneC":{"id":1}}}"#);
        Ok(())
    }
}
