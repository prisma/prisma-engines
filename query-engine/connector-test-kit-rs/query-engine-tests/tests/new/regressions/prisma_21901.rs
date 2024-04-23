use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(Enums, ScalarLists), exclude(MongoDb))]
mod prisma_21901 {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Test {
              #id(id, Int, @id)
              colors Color[]
            }
            
            enum Color {
              red
              blue
              green
            }
            "#
        };

        schema.to_owned()
    }

    // fixes https://github.com/prisma/prisma/issues/21901
    #[connector_test]
    async fn test(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(
            runner,
            r#"mutation { createOneTest(data: { id: 1, colors: ["red"] }) { colors } }"#
          ),
          @r###"{"data":{"createOneTest":{"colors":["red"]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, fmt_execute_raw(r#"TRUNCATE TABLE "prisma_21901_test"."Test" CASCADE;"#, [])),
          @r###"{"data":{"executeRaw":0}}"###
        );

        insta::assert_snapshot!(
          run_query!(
            runner,
            r#"mutation { createOneTest(data: { id: 2, colors: ["blue"] }) { colors } }"#
          ),
          @r###"{"data":{"createOneTest":{"colors":["blue"]}}}"###
        );

        Ok(())
    }
}
