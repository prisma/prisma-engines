use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), exclude(MongoDb))]
mod prisma_15177 {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Customer {
              #id(userId, Int, @id  @map("user id"))
            }"#
        };

        schema.to_owned()
    }

    // Should allow CRUD methods on a table column that has a space
    #[connector_test]
    async fn repro(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneCustomer(data: { userId: 1 }) { userId } }"#),
          @r###"{"data":{"createOneCustomer":{"userId":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyCustomer { userId } }"#),
          @r###"{"data":{"findManyCustomer":[{"userId":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneCustomer(where: { userId: 1 }, data: { userId: 2 }) { userId } }"#),
          @r###"{"data":{"updateOneCustomer":{"userId":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { deleteOneCustomer(where: { userId: 2 }) { userId } }"#),
          @r###"{"data":{"deleteOneCustomer":{"userId":2}}}"###
        );

        Ok(())
    }
}
