use query_engine_tests::*;

#[test_suite(schema(schemas::numeric_text_optional_one2m))]
mod aggregation_group_by {

    #[connector_test(exclude(MongoDb))]
    async fn group_by_no_records(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [id, float, int]) {
                      count { id }
                      float
                      sum { int }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_some_records(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 10.1, int: 5, decimal: "1.1", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 5.5, int: 0, decimal: "6.7", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 10, int: 5, decimal: "11", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 10, int: 5, decimal: "11", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{ groupByA(by: [string], orderBy: { string: asc }) { string count { string } sum { float } } }"
            ),
            @r###"{"data":{"groupByA":[{"string":"group1","count":{"string":2},"sum":{"float":15.6}},{"string":"group2","count":{"string":1},"sum":{"float":10.0}},{"string":"group3","count":{"string":1},"sum":{"float":10.0}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_rev_ordering(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 10.1, int: 5, decimal: "1.1", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 5.5, int: 0, decimal: "6.7", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 10, int: 5, decimal: "11", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 10, int: 5, decimal: "11", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{ groupByA(by: [string], orderBy: { string: desc }) { string count { string } sum { float } } }"
            ),
            @r###"{"data":{"groupByA":[{"string":"group3","count":{"string":1},"sum":{"float":10.0}},{"string":"group2","count":{"string":1},"sum":{"float":10.0}},{"string":"group1","count":{"string":2},"sum":{"float":15.6}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_multiple_ordering(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 10.1, int: 5, decimal: "1.1", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 5.5, int: 0, decimal: "6.7", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 10, int: 5, decimal: "11", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 10, int: 5, decimal: "11", string: "group3" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 5, float: 15, int: 5, decimal: "11", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: [{ string: desc }, { int: asc }]) {
                      string
                      count { string }
                      sum { float }
                      min { int }
                    }
                }"
            ),
            @r###"{"data":{"groupByA":[{"string":"group3","count":{"string":2},"sum":{"float":25.0},"min":{"int":5}},{"string":"group2","count":{"string":1},"sum":{"float":10.0},"min":{"int":5}},{"string":"group1","count":{"string":1},"sum":{"float":5.5},"min":{"int":0}},{"string":"group1","count":{"string":1},"sum":{"float":10.1},"min":{"int":5}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_take_skip(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 10.1, int: 5, decimal: "1.1", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 10, int: 5, decimal: "11", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 10, int: 5, decimal: "11", string: "group3" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 5, float: 15, int: 5, decimal: "11", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: { string: desc }, take: 1, skip: 1) {
                      string
                      count { string }
                      sum { float }
                      min { int }
                    }
                  }"
            ),
            // group3 is the first one with 2, then group2 with one, then group1 with 1.
            // group2 is returned, because group3 is skipped.
            @r###"{"data":{"groupByA":[{"string":"group2","count":{"string":1},"sum":{"float":10.0},"min":{"int":5}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: { string: desc }, take: -1, skip: 2) {
                      string
                      count { string }
                      sum { float }
                      min { int }
                    }
                  }"
            ),
            // group3 is the first one with 2, then group2 with one, then group1 with 1.
            // group3 is returned, because group1 and 2 is skipped (reverse order due to negative take).
            @r###"{"data":{"groupByA":[{"string":"group3","count":{"string":2},"sum":{"float":25.0},"min":{"int":5}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: { string: desc }, take: 2, skip: 1) {
                      string
                      count { string }
                      sum { float }
                      min { int }
                    }
                  }"
            ),
            // group3 is the first one with 2, then group2 with one, then group1 with 1.
            // group2 & 1 are returned, because group3 is skipped.
            @r###"{"data":{"groupByA":[{"string":"group2","count":{"string":1},"sum":{"float":10.0},"min":{"int":5}},{"string":"group1","count":{"string":1},"sum":{"float":10.1},"min":{"int":5}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_scalar_filters(runner: &Runner) -> TestResult<()> {
        // What this test checks: Scalar filters apply before the grouping is done,
        // changing the aggregations of the groups, not the groups directly.
        create_row(
            runner,
            r#"{ id: 1, float: 10.1, int: 5, decimal: "1.1", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 5.5, int: 0, decimal: "6.7", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 10, int: 5, decimal: "11", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 10, int: 5, decimal: "13", string: "group3" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 5, float: 15, int: 5, decimal: "10", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [string, int], orderBy: { string: desc }, where: {
                      int: 5,
                      float: { lt: 15 }
                    }) {
                      string
                      count { string }
                      sum { float }
                      min { int }
                    }
                  }"
            ),
            // Group3 has only id 4, id 5 is filtered.
            // Group2 has id 3.
            // Group1 id 1, id 2 is filtered.
            // => All groups have count 1
            @r###"{"data":{"groupByA":[{"string":"group3","count":{"string":1},"sum":{"float":10.0},"min":{"int":5}},{"string":"group2","count":{"string":1},"sum":{"float":10.0},"min":{"int":5}},{"string":"group1","count":{"string":1},"sum":{"float":10.1},"min":{"int":5}}]}}"###
        );

        Ok(())
    }

    #[connector_test(exclude(MongoDb))]
    async fn group_by_relation_filters(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 10.1, int: 5, decimal: "1.1", string: "group1", b: { create: { id: 1, field: "a" } } }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 5.5, int: 0, decimal: "6.7", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 10, int: 5, decimal: "11", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 10, int: 5, decimal: "13", string: "group3", b: { create: { id: 2, field: "b" } } }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 5, float: 15, int: 5, decimal: "10", string: "group3", b: { create: { id: 3, field: "b" } } }"#,
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
                      count { string }
                      sum { float }
                      min { int }
                    }
                  }"
            ),
            // Group3 has 2
            // Group2 has 0
            // Group1 has 1
            @r###"{"data":{"groupByA":[{"string":"group3","count":{"string":2},"sum":{"float":25.0},"min":{"int":5}},{"string":"group1","count":{"string":1},"sum":{"float":10.1},"min":{"int":5}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"{
                    groupByA(by: [string, int], orderBy: { string: desc }, where: {
                      b: { is: { field: { equals: "b" }}}
                    }) {
                      string
                      count { string }
                      sum { float }
                      min { int }
                    }
                  }"#
            ),
            // Group3 has 2 matches
            // Group2 has 0 matches
            // Group1 has 0 matches
            @r###"{"data":{"groupByA":[{"string":"group3","count":{"string":2},"sum":{"float":25.0},"min":{"int":5}}]}}"###
        );

        Ok(())
    }

    #[connector_test(exclude(MongoDb))]
    async fn group_by_ordering_count_aggregation(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 1.1, int: 2, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 1.1, int: 3, decimal: "3", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 4.0, int: 3, decimal: "4", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _count: { float: asc } }) {
                      float
                      count {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":4.0,"count":{"float":1}},{"float":1.1,"count":{"float":3}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _count: { float: desc } }) {
                      float
                      count {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"count":{"float":3}},{"float":4.0,"count":{"float":1}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_sum_aggregation(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 1.1, int: 2, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 1.1, int: 3, decimal: "3", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 4.0, int: 3, decimal: "4", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _sum: { float: asc } }) {
                      float
                      sum {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"sum":{"float":3.3}},{"float":4.0,"sum":{"float":4.0}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _sum: { float: desc } }) {
                      float
                      sum {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":4.0,"sum":{"float":4.0}},{"float":1.1,"sum":{"float":3.3}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_avg_aggregation(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 1.1, int: 2, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 1.1, int: 3, decimal: "3", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 4.0, int: 3, decimal: "4", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _avg: { float: asc } }) {
                      float
                      avg {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"avg":{"float":1.1}},{"float":4.0,"avg":{"float":4.0}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _avg: { float: desc } }) {
                      float
                      avg {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":4.0,"avg":{"float":4.0}},{"float":1.1,"avg":{"float":1.1}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_min_aggregation(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 1.1, int: 2, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 1.1, int: 3, decimal: "3", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 4.0, int: 3, decimal: "4", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _min: { float: asc } }) {
                      float
                      min {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"min":{"float":1.1}},{"float":4.0,"min":{"float":4.0}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _min: { float: desc } }) {
                      float
                      min {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":4.0,"min":{"float":4.0}},{"float":1.1,"min":{"float":1.1}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_ordering_max_aggregation(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 1.1, int: 2, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 1.1, int: 3, decimal: "3", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 4.0, int: 3, decimal: "4", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _max: { float: asc } }) {
                      float
                      max {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"max":{"float":1.1}},{"float":4.0,"max":{"float":4.0}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _max: { float: desc } }) {
                      float
                      max {
                        float
                      }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":4.0,"max":{"float":4.0}},{"float":1.1,"max":{"float":1.1}}]}}"###
        );

        Ok(())
    }

    #[connector_test(exclude(MongoDb))]
    async fn group_by_ordering_aggr_multiple_fields(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 1.1, int: 1, decimal: "3", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 3.0, int: 3, decimal: "4", string: "group3" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 5, float: 4.0, int: 4, decimal: "4", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float, int], orderBy: [{ _count: { float: desc } }, { _sum: { int: asc } }]) {
                      float
                      count { float }
                      sum { int }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"count":{"float":3},"sum":{"int":3}},{"float":3.0,"count":{"float":1},"sum":{"int":3}},{"float":4.0,"count":{"float":1},"sum":{"int":4}}]}}"###
        );

        Ok(())
    }

    #[connector_test(exclude(MongoDb))]
    async fn group_by_ordering_aggr_and_having(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 1.1, int: 1, decimal: "3", string: "group2" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 4, float: 3.0, int: 3, decimal: "4", string: "group3" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 5, float: 4.0, int: 4, decimal: "4", string: "group3" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float, int], orderBy: [{ _count: { float: desc } }, { _sum: { int: asc } }], having: { float: { lt: 4 } }) {
                      float
                      count { float }
                      sum { int }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"float":1.1,"count":{"float":3},"sum":{"int":3}},{"float":3.0,"count":{"float":1},"sum":{"int":3}}]}}"###
        );

        Ok(())
    }
    /// Order by aggregation without selection the aggregated field
    #[connector_test]
    async fn group_by_ordering_aggr_without_selecting(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 2, float: 1.1, int: 1, decimal: "11", string: "group1" }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{ id: 3, float: 1.1, int: 1, decimal: "3", string: "group2" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "{
                    groupByA(by: [float], orderBy: { _count: { float: desc } }) {
                      sum { int }
                    }
                  }"
            ),
            @r###"{"data":{"groupByA":[{"sum":{"int":3}}]}}"###
        );

        Ok(())
    }

    /// Error cases

    #[connector_test]
    async fn group_by_without_by_selection(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "query { groupByA(by: []) { string } }",
            2019,
            "At least one selection is required for the `by` argument."
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_mismatch_by_args_query_sel(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "query { groupByA(by: [int]) { string count { string } sum { float } } }",
            2019,
            "Every selected scalar field that is not part of an aggregation must be included in the by-arguments of the query. Missing fields: string"
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_by_args_order_by(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "query { groupByA(by: [int], orderBy: { string: desc }) { count { int } sum { float } } }",
            2019,
            "Every field used for orderBy must be included in the by-arguments of the query. Missing fields: string"
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_empty_aggregation_selection(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "query { groupByA(by: [string]) { sum } }",
            2009,
            "Expected a minimum of 1 fields to be present, got 0."
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneA(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
