use super::setup;

use query_engine_tests::*;

#[test_suite(schema(setup::common_types))]
mod having_filter {
    use super::setup;
    use query_engine_tests::run_query;

    #[connector_test]
    async fn basic_having_filter(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{ id: 1, string: "group1", string2: "group1", int: 1, int2: 1 }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 2, string: "group1", string2: "group2", int: 4, int2: 2 }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 3, string: "group2", string2: "group2", int: 2, int2: 2 }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 4, string: "group3", string2: "group2", int: 3, int2: 4 }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { groupByTestModel(by: [string, string2], having: {
            string: { equals: { _ref: "string2" } }
          }) {
            string
            string2
          }
        }"#),
          @r###"{"data":{"groupByTestModel":[{"string":"group1","string2":"group1"},{"string":"group2","string2":"group2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { groupByTestModel(by: [string, int], having: {
              string: { _count: { equals: { _ref: "int" } } }
            }) {
              string
              int
              _count { string }
            }
          }"#),
          @r###"{"data":{"groupByTestModel":[{"string":"group1","int":1,"_count":{"string":1}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { groupByTestModel(by: [string, int, int2], having: {
                int: { _max: { equals: { _ref: "int2" } } }
              }) {
                string
                int2
                _max { int }
              }
            }"#),
          @r###"{"data":{"groupByTestModel":[{"string":"group2","int2":2,"_max":{"int":2}},{"string":"group1","int2":1,"_max":{"int":1}}]}}"###
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
