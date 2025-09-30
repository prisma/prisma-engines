use query_engine_tests::*;

// validates fix for
// https://github.com/prisma/prisma-client/issues/933

#[test_suite(schema(schema))]
mod prisma_933_spec {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Buyer {
                #id(buyer_id, Int, @id)
                name     String?
                #m2m(sales, Sale[], sale_id, Int)
              }

              model Sale {
                #id(sale_id, Int, @id)
                #m2m(buyers, Buyer[], buyer_id, Int)
              }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn prisma_933(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyBuyer {
              sales {
                buyers {
                  buyer_id
                }
              }
            }
          }"#),
          @r###"{"data":{"findManyBuyer":[{"sales":[{"buyers":[{"buyer_id":1}]},{"buyers":[{"buyer_id":1}]}]}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ buyer_id: 1, name: "Foo", sales: { create: [{ sale_id: 1 }, { sale_id: 2 }] }}"#,
        )
        .await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneBuyer(data: {data}) {{ buyer_id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
