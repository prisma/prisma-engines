use query_engine_tests::*;

#[test_suite(schema(schema))]
mod order_by_mutation {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            "model Foo {
              #id(id, Int, @id)
              test String?
    
              bars Bar[]
          }
    
          model Bar {
              #id(id, Int, @id)
              quantity   Int
              orderField Int?
              foo_id     Int
    
              foo Foo @relation(fields: [foo_id], references: [id])
          }"
        };

        schema.to_owned()
    }

    // "Using a field in the order by that is not part of the selected fields" should "work"
    #[connector_test]
    async fn order_by_not_selected(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"
            mutation {
                createOneFoo(
                  data: {
                    id: 1,
                    bars: {
                      create: [
                        { id: 1, quantity: 1, orderField: 1}
                        { id: 2, quantity: 2, orderField: 2}
                      ]
                    }
                  }
                ) {
                  test
                  bars(take: 1, orderBy: { orderField: desc }) {
                    quantity
                  }
                }
              }
            "#),
          @r###"{"data":{"createOneFoo":{"test":null,"bars":[{"quantity":2}]}}}"###
        );

        Ok(())
    }
}
