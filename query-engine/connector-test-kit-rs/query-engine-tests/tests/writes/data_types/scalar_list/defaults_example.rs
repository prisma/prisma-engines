use query_engine_tests::*;

#[test_suite(capabilities(ScalarLists))]
mod example {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn a() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
              #id(id, Int, @id)
              dateTimes DateTime[] @default(["2019-07-31T23:59:01.000Z", "2012-07-31T23:59:01.000Z"])
            }
            "#
        };

        schema.to_owned()
    }

    fn b() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
            #id(id, Int, @id)
            dateTimes DateTime[]
          }
          "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(a))]
    async fn test_a(runner: Runner) -> TestResult<()> {
        // Driver level error
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneScalarModel(data: { id: 1 }) { dateTimes } }"#),
          @r###"doesn't matter"###
        );

        Ok(())
    }

    #[connector_test(schema(a))]
    async fn test_a_2(runner: Runner) -> TestResult<()> {
        // Driver level error, again
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneScalarModel(data: { id: 1, dateTimes: ["2019-07-31T23:59:01.000Z", "2012-07-31T23:59:01.000Z"]  }) { dateTimes } }"#),
          @r###"doesn't matter"###
        );

        Ok(())
    }

    #[connector_test(schema(b))]
    async fn test_b(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneScalarModel(data: { id: 1, dateTimes: ["2019-07-31T23:59:01.000Z", "2012-07-31T23:59:01.000Z"] }) { dateTimes } }"#),
          @r###"{"data":{"createOneScalarModel":{"dateTimes":["2019-07-31T23:59:01.000Z","2012-07-31T23:59:01.000Z"]}}}"###
        );

        Ok(())
    }
}
