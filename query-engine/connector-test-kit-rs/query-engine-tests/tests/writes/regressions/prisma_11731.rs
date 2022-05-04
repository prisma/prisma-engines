use query_engine_tests::*;

// Related issue: https://github.com/prisma/prisma/issues/11731
#[test_suite]
mod connect_or_create {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model MyModel {
                #id(id, Int, @id)
                name   String
                itemId Int?   @unique
                item   Item?  @relation(references: [id], fields: [itemId])

            }

            model Item {
                #id(id, Int, @id)
                name    String
                myModel MyModel?
            }"#
        };

        schema.to_owned()
    }

    // CreateOrConnect should not create a new model if connecting the same ID.
    #[connector_test(schema(schema))]
    async fn one2one_inlined_child(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
              createOneItem(data: { id: 1, name: "item 1" }) {
                id
              }
            }"#
        );

        run_query!(
            &runner,
            r#"mutation {
                createOneMyModel(data: {
                  id: 1,
                  name: "MyModel 1"
                  item: {
                    connect: {
                      id: 1
                    }
                  }
                }) {
                  id
                }
              }"#
        );

        // MyModel is the child in this operation as we're coming from `Item`.
        insta::assert_snapshot!(
          run_query!(&runner, r#"
            mutation {
              updateOneItem(
                where: { id: 1 }
                data: {
                  myModel: {
                    connectOrCreate: {
                      where: { itemId: 1 }
                      create: { id: 2, name: "MyModel 2" }
                    }
                  }
                }
              ) {
                id
                name
                myModel {
                  id
                  name
                }
              }
            }"#),
          @r###""###
        );

        Ok(())
    }
}
