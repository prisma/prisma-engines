// Rules for writing tests:
// - mod name + test name have to be unique in name across all test suites.
// - tests must be annotated with `connector_test`
// - test modules can be annotated with `test_suite`. you get some niceties like imports and the ability to define
// - you can use ONE OF `only` or `exclude` to scope connectors.
//    - if you use none, the test is valid for all connectors.
//
// Notes:
// - Allow dead code should be set?

use query_test_macros::{connector_test, test_suite};

pub mod schemas {
    // Wild idea: validate schemas at compile time
    pub fn some_common_schema() -> String {
        "model C {
            id Int @id
            field String?
        }"
        .to_owned()
    }
}

// The mod name dictates the db name. If the name is `some_spec`
// then, for example, the MySQL db should be (similar to) `some_spec` as well.
// #[cfg(test)]
// #[before_each(before_each_handler)] // Hook to run before each test.
// #[schema(schema_handler)] // Schema for all contained tests. Allows us to cache runners maybe.
#[test_suite(schema(schemas::some_common_schema), only(Postgres(9)))]
mod some_spec {

    // These imports are required if no `#[test_suite(...)]` is used
    // use super::*;
    // use query_tests_setup::*;
    // use std::convert::TryFrom;

    // fn before_each_handler(runner: &Runner) {
    //     // Maybe we don't need this.
    //     runner.truncate_data(); // Actually, this should always happen for a connector test.
    //     test_data(); // This can also be done in each test manually or by convention.
    // }

    // Handler that returns a schema template to use for rendering.
    // Template rendering can be bypassed by simply not using the template strings.
    // Common schema handlers to use should be in a central place.
    fn schema_handler() -> String {
        // #id(id, Int, @id)
        "model A {
            id Int @id
            field String?
        }"
        .to_owned()
    }

    //(suite = "some_speccc", schema(schema_handler), only(SqlServer, Postgres))
    #[connector_test(suite = "named_suite", schema(schema_handler), only(Postgres(10)))]
    async fn ideal_api_test(runner: &Runner) -> TestResult<()> {
        let result = runner
            .query(
                r#"
            mutation {
                createOneA(data: { id: 1, field: "1"}) { id }
            }
        "#,
            )
            .await?;

        assert_eq!(result.to_string(), r#"{"data":{"createOneA":{"id":1}}}"#);
        Ok(())
    }

    // #[connector_test(suite = "some_speccc", schema(schema_handler), only(SqlServer, Postgres))]
    // async fn other_test(runner: &Runner) -> TestResult<()> {
    //     let result = runner
    //         .query(
    //             r#"
    //         mutation {
    //             createOneA(data: { id: 1, field: "1"}) { id }
    //         }
    //     "#,
    //         )
    //         .await?;

    //     assert_eq!(result.to_string(), r#"{"data":{"createOneA":{"id":1}}}"#);
    //     Ok(())
    // }
}
