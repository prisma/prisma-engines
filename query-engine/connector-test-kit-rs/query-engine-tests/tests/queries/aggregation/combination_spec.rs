use query_engine_tests::*;

#[test_suite(schema(schema))]
mod combinations {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"
              model Item {
                #id(id, String, @id, @default(cuid()))
                float Float   @map("db_float")
                int   Int     @map("db_int")
                dec   Decimal @map("db_dec")
              }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn no_records(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, indoc! { r#"
              {
                aggregateItem {
                  _count { _all }
                  _sum {
                    float
                    int
                    dec
                  }
                  _avg {
                    float
                    int
                    dec
                  }
                  _min {
                    float
                    int
                    dec
                  }
                  _max {
                    float
                    int
                    dec
                  }
                }
              }
            "# }),
            @r###"{"data":{"aggregateItem":{"_count":{"_all":0},"_sum":{"float":null,"int":null,"dec":null},"_avg":{"float":null,"int":null,"dec":null},"_min":{"float":null,"int":null,"dec":null},"_max":{"float":null,"int":null,"dec":null}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn some_records(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ float: 5.5, int: 5, dec: "5.5" }"#).await?;
        create_row(&runner, r#"{ float: 4.5, int: 10, dec: "4.5" }"#).await?;

        insta::assert_snapshot!(
            run_query!(&runner, indoc! { r#"
              {
                aggregateItem {
                  _count { _all }
                  _sum {
                    float
                    int
                    dec
                  }
                  _avg {
                    float
                    int
                    dec
                  }
                  _min {
                    float
                    int
                    dec
                  }
                  _max {
                    float
                    int
                    dec
                  }
                }
              }
            "# }),
            @r###"{"data":{"aggregateItem":{"_count":{"_all":2},"_sum":{"float":10.0,"int":15,"dec":"10"},"_avg":{"float":5.0,"int":7.5,"dec":"5"},"_min":{"float":4.5,"int":5,"dec":"4.5"},"_max":{"float":5.5,"int":10,"dec":"5.5"}}}}"###
        );

        Ok(())
    }

    // Mongo precision issue.
    #[connector_test(exclude(MongoDB))]
    async fn with_query_args(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: "1", float: 5.5, int: 5, dec: "5.5" }"#).await?;
        create_row(&runner, r#"{ id: "2", float: 4.5, int: 10, dec: "4.5" }"#).await?;
        create_row(&runner, r#"{ id: "3", float: 1.5, int: 2, dec: "1.5" }"#).await?;
        create_row(&runner, r#"{ id: "4", float: 0, int: 1, dec: "0"  }"#).await?;

        insta::assert_snapshot!(
            run_query!(&runner, indoc! { r#"
              {
                aggregateItem(take: 2) {
                  _count { _all }
                  _sum {
                    float
                    int
                    dec
                  }
                  _avg {
                    float
                    int
                    dec
                  }
                  _min {
                    float
                    int
                    dec
                  }
                  _max {
                    float
                    int
                    dec
                  }
                }
              }
            "# }),
            @r###"{"data":{"aggregateItem":{"_count":{"_all":2},"_sum":{"float":10.0,"int":15,"dec":"10"},"_avg":{"float":5.0,"int":7.5,"dec":"5"},"_min":{"float":4.5,"int":5,"dec":"4.5"},"_max":{"float":5.5,"int":10,"dec":"5.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, indoc! { r#"
              {
                aggregateItem(take: 5) {
                  _count { _all }
                  _sum {
                    float
                    int
                    dec
                  }
                  _avg {
                    float
                    int
                    dec
                  }
                  _min {
                    float
                    int
                    dec
                  }
                  _max {
                    float
                    int
                    dec
                  }
                }
              }
            "# }),
            @r###"{"data":{"aggregateItem":{"_count":{"_all":4},"_sum":{"float":11.5,"int":18,"dec":"11.5"},"_avg":{"float":2.875,"int":4.5,"dec":"2.875"},"_min":{"float":0.0,"int":1,"dec":"0"},"_max":{"float":5.5,"int":10,"dec":"5.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, indoc! { r#"
              {
                aggregateItem(take: -5) {
                  _count { _all }
                  _sum {
                    float
                    int
                    dec
                  }
                  _avg {
                    float
                    int
                    dec
                  }
                  _min {
                    float
                    int
                    dec
                  }
                  _max {
                    float
                    int
                    dec
                  }
                }
              }
            "# }),
            @r###"{"data":{"aggregateItem":{"_count":{"_all":4},"_sum":{"float":11.5,"int":18,"dec":"11.5"},"_avg":{"float":2.875,"int":4.5,"dec":"2.875"},"_min":{"float":0.0,"int":1,"dec":"0"},"_max":{"float":5.5,"int":10,"dec":"5.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, indoc! { r#"
              {
                aggregateItem(where: { id: { gt: "2" }}) {
                  _count { _all }
                  _sum {
                    float
                    int
                    dec
                  }
                  _avg {
                    float
                    int
                    dec
                  }
                  _min {
                    float
                    int
                    dec
                  }
                  _max {
                    float
                    int
                    dec
                  }
                }
              }
            "# }),
            @r###"{"data":{"aggregateItem":{"_count":{"_all":2},"_sum":{"float":1.5,"int":3,"dec":"1.5"},"_avg":{"float":0.75,"int":1.5,"dec":"0.75"},"_min":{"float":0.0,"int":1,"dec":"0"},"_max":{"float":1.5,"int":2,"dec":"1.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, indoc! { r#"
              {
                aggregateItem(skip: 2) {
                  _count { _all }
                  _sum {
                    float
                    int
                    dec
                  }
                  _avg {
                    float
                    int
                    dec
                  }
                  _min {
                    float
                    int
                    dec
                  }
                  _max {
                    float
                    int
                    dec
                  }
                }
              }
            "# }),
            @r###"{"data":{"aggregateItem":{"_count":{"_all":2},"_sum":{"float":1.5,"int":3,"dec":"1.5"},"_avg":{"float":0.75,"int":1.5,"dec":"0.75"},"_min":{"float":0.0,"int":1,"dec":"0"},"_max":{"float":1.5,"int":2,"dec":"1.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, indoc! { r#"
              {
                aggregateItem(cursor: { id: "3" }) {
                  _count { _all }
                  _sum {
                    float
                    int
                    dec
                  }
                  _avg {
                    float
                    int
                    dec
                  }
                  _min {
                    float
                    int
                    dec
                  }
                  _max {
                    float
                    int
                    dec
                  }
                }
              }
            "# }),
            @r###"{"data":{"aggregateItem":{"_count":{"_all":2},"_sum":{"float":1.5,"int":3,"dec":"1.5"},"_avg":{"float":0.75,"int":1.5,"dec":"0.75"},"_min":{"float":0.0,"int":1,"dec":"0"},"_max":{"float":1.5,"int":2,"dec":"1.5"}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn unstable_cursor(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: "1", float: 5.5, int: 5, dec: "5.5" }"#).await?;
        create_row(&runner, r#"{ id: "2", float: 4.5, int: 10, dec: "4.5" }"#).await?;
        create_row(&runner, r#"{ id: "3", float: 1.5, int: 2, dec: "1.5" }"#).await?;
        create_row(&runner, r#"{ id: "4", float: 0, int: 1, dec: "0"  }"#).await?;

        assert_error!(
            runner,
            r#"{
                aggregateItem(cursor: { id: "3" }, orderBy: { float: asc }) {
                  _count { _all }
                }
              }
            "#,
            2019,
            "Unable to process combination of query arguments for aggregation query"
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneItem(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();

        Ok(())
    }
}
