use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod prisma_16760 {
    fn schema() -> String {
        indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              list        String[]
            }"#
        }
        .to_owned()
    }

    #[connector_test(capabilities(ScalarLists, EnumArrayPush))]
    async fn regression(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { upsertOneTestModel(
            where: { id: 1 },
            create: { id: 1, list: { set: ["foo"] } },
            update: { list: { push: ["bar"] } },
          ) { id } }"#),
          @r###"{"data":{"upsertOneTestModel":{"id":1}}}"###
        );

        Ok(())
    }
}
