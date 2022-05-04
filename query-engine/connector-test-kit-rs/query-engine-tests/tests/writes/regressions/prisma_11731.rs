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
                uniq    String   @unique
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
              createOneItem(data: { id: 1, uniq: "item 1" }) {
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
                uniq
                myModel {
                  id
                  name
                }
              }
            }"#),
          @r###"{"data":{"updateOneItem":{"id":1,"uniq":"item 1","myModel":{"id":1,"name":"MyModel 1"}}}}"###
        );

        // There's only one MyModel
        insta::assert_snapshot!(
          run_query!(&runner, r#"
            {
              findManyMyModel {
                id
                item {
                  id
                }
              }
            }"#),
          @r###"{"data":{"findManyMyModel":[{"id":1,"item":{"id":1}}]}}"###
        );

        Ok(())
    }

    // CreateOrConnect should not create a new model if connecting the same ID.
    #[connector_test(schema(schema))]
    async fn one2one_inlined_parent(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
              createOneItem(data: { id: 1, uniq: "item 1" }) {
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
              updateOneMyModel(
                where: { id: 1 }
                data: {
                  item: {
                    connectOrCreate: {
                      where: { uniq: "item 1" }
                      create: { id: 2, uniq: "item 2" }
                    }
                  }
                }
              ) {
                id
                name
                item {
                  id
                  uniq
                }
              }
            }"#),
          @r###"{"data":{"updateOneMyModel":{"id":1,"name":"MyModel 1","item":{"id":1,"uniq":"item 1"}}}}"###
        );

        // There's only one item
        insta::assert_snapshot!(
          run_query!(&runner, r#"
            {
              findManyItem {
                id
                myModel {
                  id
                }
              }
            }"#),
          @r###"{"data":{"findManyItem":[{"id":1,"myModel":{"id":1}}]}}"###
        );

        Ok(())
    }
}
