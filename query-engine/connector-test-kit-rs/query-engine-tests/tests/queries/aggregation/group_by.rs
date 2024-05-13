use query_engine_tests::*;

#[test_suite(schema(schemas::numeric_text_optional_one2m))]
mod aggregation_group_by {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test]
    async fn group_by_no_records(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [id, float, int]) {
                      _count { id }
                      float
                      _sum { int }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_some_records(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 10.1, int: 5, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 5.5, int: 0, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 10, int: 5, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 10, int: 5, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{ groupByA(by: [string], orderBy: { string: asc }) { string _count { string } _sum { float } } }"
            ),
            @r###"{"data":{"groupByA":[{"string":"group1","_count":{"string":2},"_sum":{"float":15.6}},{"string":"group2","_count":{"string":1},"_sum":{"float":10.0}},{"string":"group3","_count":{"string":1},"_sum":{"float":10.0}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_rev_ordering(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 10.1, int: 5, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 5.5, int: 0, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 10, int: 5, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 10, int: 5, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{ groupByA(by: [string], orderBy: { string: desc }) { string _count { string } _sum { float } } }"
            ),
            @r###"{"data":{"groupByA":[{"string":"group3","_count":{"string":1},"_sum":{"float":10.0}},{"string":"group2","_count":{"string":1},"_sum":{"float":10.0}},{"string":"group1","_count":{"string":2},"_sum":{"float":15.6}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_multiple_ordering(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 10.1, int: 5, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 5.5, int: 0, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 10, int: 5, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 10, int: 5, string: "group3" }"#).await?;
        create_row(&runner, r#"{ id: 5, float: 15, int: 5, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: [{ string: desc }, { int: asc }]) {
                      string
                      _count { string }
                      _sum { float }
                      _min { int }
                    }
                }"
            ),
            @r###"{"data":{"groupByA":[{"string":"group3","_count":{"string":2},"_sum":{"float":25.0},"_min":{"int":5}},{"string":"group2","_count":{"string":1},"_sum":{"float":10.0},"_min":{"int":5}},{"string":"group1","_count":{"string":1},"_sum":{"float":5.5},"_min":{"int":0}},{"string":"group1","_count":{"string":1},"_sum":{"float":10.1},"_min":{"int":5}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_take_skip(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 10.1, int: 5, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 10, int: 5, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 10, int: 5, string: "group3" }"#).await?;
        create_row(&runner, r#"{ id: 5, float: 15, int: 5, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: { string: desc }, take: 1, skip: 1) {
                      string
                      _count { string }
                      _sum { float }
                      _min { int }
                    }
                  }"
            ),
            // group3 is the first one with 2, then group2 with one, then group1 with 1.
            // group2 is returned, because group3 is skipped.
            @r###"{"data":{"groupByA":[{"string":"group2","_count":{"string":1},"_sum":{"float":10.0},"_min":{"int":5}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: { string: desc }, take: -1, skip: 2) {
                      string
                      _count { string }
                      _sum { float }
                      _min { int }
                    }
                  }"
            ),
            // group3 is the first one with 2, then group2 with one, then group1 with 1.
            // group3 is returned, because group1 and 2 is skipped (reverse order due to negative take).
            @r###"{"data":{"groupByA":[{"string":"group3","_count":{"string":2},"_sum":{"float":25.0},"_min":{"int":5}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: { string: desc }, take: 2, skip: 1) {
                      string
                      _count { string }
                      _sum { float }
                      _min { int }
                    }
                  }"
            ),
            // group3 is the first one with 2, then group2 with one, then group1 with 1.
            // group2 & 1 are returned, because group3 is skipped.
            @r###"{"data":{"groupByA":[{"string":"group2","_count":{"string":1},"_sum":{"float":10.0},"_min":{"int":5}},{"string":"group1","_count":{"string":1},"_sum":{"float":10.1},"_min":{"int":5}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_scalar_filters(runner: Runner) -> TestResult<()> {
        // What this test checks: Scalar filters apply before the grouping is done,
        // changing the aggregations of the groups, not the groups directly.
        create_row(&runner, r#"{ id: 1, float: 10.1, int: 5, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 5.5, int: 0, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 10, int: 5, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 10, int: 5, string: "group3" }"#).await?;
        create_row(&runner, r#"{ id: 5, float: 15, int: 5, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: { string: desc }, where: {
                      int: 5,
                      float: { lt: 15 }
                    }) {
                      string
                      _count { string }
                      _sum { float }
                      _min { int }
                    }
                  }"
            ),
            // Group3 has only id 4, id 5 is filtered.
            // Group2 has id 3.
            // Group1 id 1, id 2 is filtered.
            // => All groups have count 1
            @r###"{"data":{"groupByA":[{"string":"group3","_count":{"string":1},"_sum":{"float":10.0},"_min":{"int":5}},{"string":"group2","_count":{"string":1},"_sum":{"float":10.0},"_min":{"int":5}},{"string":"group1","_count":{"string":1},"_sum":{"float":10.1},"_min":{"int":5}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_relation_filters(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{ id: 1, float: 10.1, int: 5, string: "group1", b: { create: { id: 1, field: "a" } } }"#,
        )
        .await?;
        create_row(&runner, r#"{ id: 2, float: 5.5, int: 0, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 10, int: 5, string: "group2" }"#).await?;
        create_row(
            &runner,
            r#"{ id: 4, float: 10, int: 5, string: "group3", b: { create: { id: 2, field: "b" } } }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 5, float: 15, int: 5, string: "group3", b: { create: { id: 3, field: "b" } } }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: { string: desc }, where: {
                      b: { isNot: null }
                    }) {
                      string
                      _count { string }
                      _sum { float }
                      _min { int }
                    }
                  }"
            ),
            // Group3 has 2
            // Group2 has 0
            // Group1 has 1
            @r###"{"data":{"groupByA":[{"string":"group3","_count":{"string":2},"_sum":{"float":25.0},"_min":{"int":5}},{"string":"group1","_count":{"string":1},"_sum":{"float":10.1},"_min":{"int":5}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"{
                    groupByA(by: [string, int], orderBy: { string: desc }, where: {
                      b: { is: { field: { equals: "b" }}}
                    }) {
                      string
                      _count { string }
                      _sum { float }
                      _min { int }
                    }
                  }"#
            ),
            // Group3 has 2 matches
            // Group2 has 0 matches
            // Group1 has 0 matches
            @r###"{"data":{"groupByA":[{"string":"group3","_count":{"string":2},"_sum":{"float":25.0},"_min":{"int":5}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_count_aggregation(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 1.1, int: 2, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 1.1, int: 3, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 4.0, int: 3, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _count: { float: asc } }) {
                      float
                      _count {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":4.0,"_count":{"float":1}},{"float":1.1,"_count":{"float":3}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _count: { float: desc } }) {
                      float
                      _count {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"_count":{"float":3}},{"float":4.0,"_count":{"float":1}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_sum_aggregation(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 1.1, int: 2, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 1.1, int: 3, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 4.0, int: 3, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _sum: { float: asc } }) {
                      float
                      _sum {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"_sum":{"float":3.3}},{"float":4.0,"_sum":{"float":4.0}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _sum: { float: desc } }) {
                      float
                      _sum {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":4.0,"_sum":{"float":4.0}},{"float":1.1,"_sum":{"float":3.3}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_avg_aggregation(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 1.1, int: 2, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 1.1, int: 3, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 4.0, int: 3, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _avg: { float: asc } }) {
                      float
                      _avg {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"_avg":{"float":1.1}},{"float":4.0,"_avg":{"float":4.0}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _avg: { float: desc } }) {
                      float
                      _avg {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":4.0,"_avg":{"float":4.0}},{"float":1.1,"_avg":{"float":1.1}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_min_aggregation(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 1.1, int: 2, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 1.1, int: 3, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 4.0, int: 3, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _min: { float: asc } }) {
                      float
                      _min {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"_min":{"float":1.1}},{"float":4.0,"_min":{"float":4.0}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _min: { float: desc } }) {
                      float
                      _min {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":4.0,"_min":{"float":4.0}},{"float":1.1,"_min":{"float":1.1}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_max_aggregation(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 1.1, int: 2, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 1.1, int: 3, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 4.0, int: 3, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _max: { float: asc } }) {
                      float
                      _max {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"_max":{"float":1.1}},{"float":4.0,"_max":{"float":4.0}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _max: { float: desc } }) {
                      float
                      _max {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":4.0,"_max":{"float":4.0}},{"float":1.1,"_max":{"float":1.1}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_aggr_multiple_fields(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 1.1, int: 1, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 3.0, int: 3, string: "group3" }"#).await?;
        create_row(&runner, r#"{ id: 5, float: 4.0, int: 4, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float, int], orderBy: [{ _count: { float: desc } }, { _sum: { int: asc } }]) {
                      float
                      _count { float }
                      _sum { int }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"_count":{"float":3},"_sum":{"int":3}},{"float":3.0,"_count":{"float":1},"_sum":{"int":3}},{"float":4.0,"_count":{"float":1},"_sum":{"int":4}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_aggr_and_having(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 1.1, int: 1, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 3.0, int: 3, string: "group3" }"#).await?;
        create_row(&runner, r#"{ id: 5, float: 4.0, int: 4, string: "group3" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float, int], orderBy: [{ _count: { float: desc } }, { _sum: { int: asc } }], having: { float: { lt: 4 } }) {
                      float
                      _count { float }
                      _sum { int }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"_count":{"float":3},"_sum":{"int":3}},{"float":3.0,"_count":{"float":1},"_sum":{"int":3}}]}}"###
        );

        Ok(())
    }
    /// Order by aggregation without selection the aggregated field
    #[connector_test]
    async fn group_by_ordering_aggr_without_selecting(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 1.1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 1.1, int: 1, string: "group2" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _count: { float: desc } }) {
                      _sum { int }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"_sum":{"int":3}}]}}"###
        );

        Ok(())
    }

    fn schema_21789() -> String {
        let schema = indoc! {
            r#"model Test {
              #id(id, Int, @id)
              group Int
              color Color
            }
            
            enum Color {
              blue
              red
              green
            }
            "#
        };

        schema.to_owned()
    }

    // regression test for https://github.com/prisma/prisma/issues/21789
    #[connector_test(schema(schema_21789), only(Postgres, CockroachDB))]
    async fn regression_21789(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneTest(data: { id: 1, group: 1, color: "red" }) { id } }"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneTest(data: { id: 2, group: 2, color: "green" }) { id } }"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneTest(data: { id: 3, group: 1, color: "blue" }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ aggregateTest { _max { color } _min { color } } }"#),
          @r###"{"data":{"aggregateTest":{"_max":{"color":"green"},"_min":{"color":"blue"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ groupByTest(by: [group], orderBy: { group: asc }) { group _max { color } _min { color } } }"#),
          @r###"{"data":{"groupByTest":[{"group":1,"_max":{"color":"red"},"_min":{"color":"blue"}},{"group":2,"_max":{"color":"green"},"_min":{"color":"green"}}]}}"###
        );

        Ok(())
    }

    /// Error cases

    #[connector_test]
    async fn group_by_without_by_selection(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "query { groupByA(by: []) { string } }",
            2019,
            "At least one selection is required for the `by` argument."
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_mismatch_by_args_query_sel(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "query { groupByA(by: [int]) { string _count { string } _sum { float } } }",
            2019,
            "Every selected scalar field that is not part of an aggregation must be included in the by-arguments of the query. Missing fields: string"
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_by_args_order_by(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "query { groupByA(by: [int], orderBy: { string: desc }) { _count { int } _sum { float } } }",
            2019,
            "Every field used for orderBy must be included in the by-arguments of the query. Missing fields: string"
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_empty_aggregation_selection(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "query { groupByA(by: [string]) { _sum } }",
            2009,
            "Expected a minimum of 1 field to be present, got 0"
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneA(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
