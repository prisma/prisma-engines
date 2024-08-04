use query_engine_tests::*;

#[test_suite(schema(schema))]
mod prisma_17103 {
    fn schema() -> String {
        let schema = indoc! {
            r#"model A {
                #id(id, Int, @id)
              
                b   B? @relation(fields: [bId], references: [id])
                bId Int?
              }
              
              model B {
                #id(id, Int, @id)
                a   A[]
              }
              "#
        };

        schema.to_owned()
    }

    // On PlanetScale, this fails with:
    // "Expected 1 records to be connected after connect operation on one-to-many relation 'AToB', found 0."
    #[connector_test(exclude(Vitess("planetscale.js", "planetscale.js.wasm")))]
    async fn regression(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
            createOneA(data: { id: 1, b: { create: { id: 1 } } }) {
              id
            }
          }
          "#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneB(where: { id: 1 }, data: { a: { connect: { id: 1 } } }) { id } }"#),
          @r###"{"data":{"updateOneB":{"id":1}}}"###
        );

        Ok(())
    }
}
