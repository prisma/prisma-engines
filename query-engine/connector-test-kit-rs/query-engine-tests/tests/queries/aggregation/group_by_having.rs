use query_engine_tests::*;

// Testing assumptions
// - Grouping on fields itself works (as tested in the group_by.rs spec).
// - The above means we also don't need to test combinations except for what is required by the rules to make it work.
// - It also means we don't need to test the selection of aggregates extensively beyond the need to sanity check the group filter.
// - We don't need to check every single filter operation, as it's ultimately the same code path just with different
//   operators applied. For a good confidence, we choose `equals`, `in`, `not equals`, `endsWith` (where applicable).
#[test_suite(schema(schemas::common_text_and_numeric_types_optional))]
mod aggregation_group_by_having {
    use query_engine_tests::{assert_error, assert_query_many, run_query, Runner};

    // This is just basic confirmation that scalar filters are applied correctly.
    // The assumption is that we don't need to test all normal scalar filters as they share the exact same code path
    // and are extracted and applied exactly as the already tested ones. This also extends to AND/OR/NOT combinators.
    // Consequently, subsequent tests in this file will deal exclusively with the newly added aggregation filters.
    #[connector_test]
    async fn basic_having_scalar_filter(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{ id: 1, float: 10.1, int: 5, decimal: "1.1", bInt: "12", string: "group1" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 2, float: 5.5, int: 0, decimal: "6.7", bInt: "3", string: "group1" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 3, float: 10, int: 5, decimal: "11", bInt: "3", string: "group2" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 4, float: 10, int: 5, decimal: "11", bInt: "3", string: "group3" }"#,
        )
        .await?;

        // Group [string, int] produces:
        // group1, 5
        // group1, 0
        // group2, 5
        // group3, 5
        assert_query_many!(
            &runner,
            r#"query { groupByTestModel(by: [string, int], having: {
                string: { in: ["group1", "group2"] }
                int: 5
              }) {
                string
                int
                _count { _all }
                _sum { int }
              }
            }"#,
            vec![
                r#"{"data":{"groupByTestModel":[{"string":"group1","int":5,"_count":{"_all":1},"_sum":{"int":5}},{"string":"group2","int":5,"_count":{"_all":1},"_sum":{"int":5}}]}}"#, // SQL
                r#"{"data":{"groupByTestModel":[{"string":"group2","int":5,"_count":{"_all":1},"_sum":{"int":5}},{"string":"group1","int":5,"_count":{"_all":1},"_sum":{"int":5}}]}}"# // Mongo
            ]
        );

        Ok(())
    }

    #[connector_test]
    async fn having_count_scalar_filter(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, int: 1, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, int: 2, string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, int: 3, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 5, string: "group3" }"#).await?;
        create_row(&runner, r#"{ id: 6, string: "group3" }"#).await?;

        // Group 1 has 2, 2 has 1, 3 has 0
        insta::assert_snapshot!(
            run_query!(
                runner,
                "query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    int: {
                      _count: {
                        equals: 2
                      }
                    }
                  }) {
                    string
                    _count {
                      int
                    }
                  }
                }"
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group1","_count":{"int":2}}]}}"###
        );

        // Group 2 and 3 returned
        insta::assert_snapshot!(
            run_query!(
                runner,
                "query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    int: {
                      _count: {
                        not: { equals: 2 }
                      }
                    }
                  }) {
                    string
                    _count {
                      int
                    }
                  }
                }"
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group2","_count":{"int":1}},{"string":"group3","_count":{"int":0}}]}}"###
        );

        // Group 1 and 3 returned
        insta::assert_snapshot!(
            run_query!(
                runner,
                "query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    int: {
                      _count: {
                        in: [0, 2]
                      }
                    }
                  }) {
                    string
                    _count {
                      int
                    }
                  }
                }"
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group1","_count":{"int":2}},{"string":"group3","_count":{"int":0}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn having_avg_scalar_filter(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, int: 10, decimal: "10", string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 2, int: 6, decimal: "6", string: "group1" }"#).await?;
        create_row(&runner, r#"{ id: 3, int: 3, decimal: "5", string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 4, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 5, string: "group3" }"#).await?;
        create_row(&runner, r#"{ id: 6, string: "group3" }"#).await?;

        // Group 1 has 8, 2 has 5, 3 has 0
        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    decimal: {
                      _avg: {
                        equals: "8.0"
                      }
                    }
                  }) {
                    string
                    _avg {
                      decimal
                    }
                  }
                }"#
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group1","_avg":{"decimal":"8"}}]}}"###
        );

        // Group 2 and 3 returned (3 is null)
        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    OR: [
                      { decimal: { _avg: { not: { equals: "8.0" }}}},
                      { decimal: { _avg: { equals: null }}}
                    ]}
                  ) {
                      string
                      _avg {
                        decimal
                      }
                    }
                }"#
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group2","_avg":{"decimal":"5"}},{"string":"group3","_avg":{"decimal":null}}]}}"###
        );

        // Group 1 and 3 returned
        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    decimal: {
                      _avg: {
                        in: ["8", "5"]
                      }
                    }
                  }) {
                    string
                    _avg {
                      decimal
                    }
                  }
                }"#
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group1","_avg":{"decimal":"8"}},{"string":"group2","_avg":{"decimal":"5"}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn having_sum_scalar_filter(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{ id: 1, float: 10, int: 10, decimal: "10", string: "group1" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 2, float: 6, int: 6, decimal: "6", string: "group1" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 3, float: 5, int: 5, decimal: "5", string: "group2" }"#,
        )
        .await?;
        create_row(&runner, r#"{ id: 4, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 5, string: "group3" }"#).await?;
        create_row(&runner, r#"{ id: 6, string: "group3" }"#).await?;

        // Group 1 has 16, 2 has 6, 3 has 0
        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"{ groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    float: { _sum: { equals: 16 }}
                    int: { _sum: { equals: 16 }}
                    decimal: { _sum: { equals: "16" }}
                  }) {
                    string
                    _sum {
                      float
                      int
                      decimal
                    }
                  }
                }"#
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group1","_sum":{"float":16.0,"int":16,"decimal":"16"}}]}}"###
        );

        let res = run_query!(
            runner,
            r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
              float: { _sum: { not: { equals: 16 }}}
              int: { _sum: { not: { equals: 16 }}}
              decimal: { _sum: { not: { equals: "16" }}}
            }) {
              string
              _sum {
                float
                int
                decimal
              }
            }
          }"#
        );
        // On MongoDB, having sum returns 0 where there are inexistant element
        // Contrary to SQL which returns NULL and therefore excludes group3
        match runner.connector() {
            query_engine_tests::ConnectorTag::MongoDb(_) => {
                assert_eq!(
                    res,
                    r#"{"data":{"groupByTestModel":[{"string":"group2","_sum":{"float":5.0,"int":5,"decimal":"5"}},{"string":"group3","_sum":{"float":0.0,"int":0,"decimal":"0"}}]}}"#
                )
            }
            _ => {
                assert_eq!(
                    res,
                    r#"{"data":{"groupByTestModel":[{"string":"group2","_sum":{"float":5.0,"int":5,"decimal":"5"}}]}}"#
                );
            }
        }

        // Group 1 and 2 returned
        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    float: { _sum: { in: [16, 5] }}
                    int: { _sum: { in: [16, 5] }}
                    decimal: { _sum: { in: ["16", "5"] }}
                  }) {
                    string
                    _sum {
                      float
                      int
                      decimal
                    }
                  }
                }"#
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group1","_sum":{"float":16.0,"int":16,"decimal":"16"}},{"string":"group2","_sum":{"float":5.0,"int":5,"decimal":"5"}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn having_min_scalar_filter(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{ id: 1, float: 10, int: 10, decimal: "10", string: "group1" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 2, float: 0, int: 0, decimal: "0", string: "group1" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 3, float: 0, int: 0, decimal: "0", string: "group2" }"#,
        )
        .await?;
        create_row(&runner, r#"{ id: 4, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 5, string: "group3" }"#).await?;
        create_row(&runner, r#"{ id: 6, string: "group3" }"#).await?;

        // Group 1 and 2 returned
        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    float: { _min: { equals: 0 }}
                    int: { _min: { equals: 0 }}
                    decimal: { _min: { equals: "0" }}
                  }) {
                    string
                    _min {
                      float
                      int
                      decimal
                    }
                  }
                }"#
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group1","_min":{"float":0.0,"int":0,"decimal":"0"}},{"string":"group2","_min":{"float":0.0,"int":0,"decimal":"0"}}]}}"###
        );

        let res = run_query!(
            runner,
            r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
              float: { _min: { not: { equals: 0 }}}
              int: { _min: { not: { equals: 0 }}}
              decimal: { _min: { not: { equals: "0" }}}
            }) {
              string
              _min {
                float
                int
                decimal
              }
            }
          }"#
        );

        // Since MongoDB doesn't have the concept of `NULL` `not equals 10` includes `null` records
        // The result _is_ empty on SQL though
        match runner.connector() {
            query_engine_tests::ConnectorTag::MongoDb(_) => {
                assert_eq!(
                    res,
                    r#"{"data":{"groupByTestModel":[{"string":"group3","_min":{"float":null,"int":null,"decimal":null}}]}}"#
                );
            }
            _ => {
                assert_eq!(res, r#"{"data":{"groupByTestModel":[]}}"#);
            }
        }

        // Group 1 and 2 returned
        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    float: { _min: { in: [0] }}
                    int: { _min: { in: [0] }}
                    decimal: { _min: { in: ["0"] }}
                  }) {
                    string
                    _min {
                      float
                      int
                      decimal
                    }
                  }
                }"#
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group1","_min":{"float":0.0,"int":0,"decimal":"0"}},{"string":"group2","_min":{"float":0.0,"int":0,"decimal":"0"}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn having_max_scalar_filter(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{ id: 1, float: 10, int: 10, decimal: "10", string: "group1" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 2, float: 0, int: 0, decimal: "0", string: "group1" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 3, float: 10, int: 10, decimal: "10", string: "group2" }"#,
        )
        .await?;
        create_row(&runner, r#"{ id: 4, string: "group2" }"#).await?;
        create_row(&runner, r#"{ id: 5, string: "group3" }"#).await?;
        create_row(&runner, r#"{ id: 6, string: "group3" }"#).await?;

        // Group 1 returned
        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    float: { _max: { equals: 10 }}
                    int: { _max: { equals: 10 }}
                    decimal: { _max: { equals: "10" }}
                  }) {
                    string
                    _max {
                      float
                      int
                      decimal
                    }
                  }
                }"#
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group1","_max":{"float":10.0,"int":10,"decimal":"10"}},{"string":"group2","_max":{"float":10.0,"int":10,"decimal":"10"}}]}}"###
        );

        let res = run_query!(
            runner,
            r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
              float: { _max: { not: { equals: 10 }}}
              int: { _max: { not: { equals: 10 }}}
              decimal: { _max: { not: { equals: "10" }}}
            }) {
              string
              _max {
                float
                int
                decimal
              }
            }
          }"#
        );
        // Since MongoDB doesn't have the concept of `NULL` `not equals 10` includes `null` records
        // The result _is_ empty on SQL though
        match runner.connector() {
            query_engine_tests::ConnectorTag::MongoDb(_) => {
                assert_eq!(
                    res,
                    r#"{"data":{"groupByTestModel":[{"string":"group3","_max":{"float":null,"int":null,"decimal":null}}]}}"#
                );
            }
            _ => {
                assert_eq!(res, r#"{"data":{"groupByTestModel":[]}}"#);
            }
        }

        // Group 1 and 2 returned
        insta::assert_snapshot!(
            run_query!(
                runner,
                r#"query { groupByTestModel(by: [string], orderBy: { string: asc }, having: {
                    float: { _max: { in: [10] }}
                    int: { _max: { in: [10] }}
                    decimal: { _max: { in: ["10"] }}
                  }) {
                    string
                    _max {
                      float
                      int
                      decimal
                    }
                  }
                }"#
            ),
            @r###"{"data":{"groupByTestModel":[{"string":"group1","_max":{"float":10.0,"int":10,"decimal":"10"}},{"string":"group2","_max":{"float":10.0,"int":10,"decimal":"10"}}]}}"###
        );

        Ok(())
    }

    /// Error cases

    #[connector_test]
    async fn having_filter_mismatch_selection(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "query { groupByTestModel(by: [string], having: { int: { gt: 3 } }) {
                _sum {
                  int
                }
              }
            }",
            2019,
            "Every field used in `having` filters must either be an aggregation filter or be included in the selection of the query. Missing fields: int"
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
