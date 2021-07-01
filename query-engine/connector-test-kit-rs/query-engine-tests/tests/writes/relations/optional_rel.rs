use query_engine_tests::*;

#[test_suite(schema(schema))]
mod opt_rel {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model List{
              #id(id, Int, @id)
              name   String
              todoId Int?
              todo   Todo?   @relation(fields: [todoId], references: [id])
            }

            model Todo{
              #id(id, Int, @id)
              name   String
              lists  List[]
            }"#
        };

        schema.to_owned()
    }

    // "Updating an optional relation with null" should "return an error"
    #[connector_test]
    async fn update_opt_rel_with_null_should_fail(runner: &Runner) -> TestResult<()> {
        // Setup
        insta::assert_snapshot!(
          run_query!(runner, r#" mutation {
            createOneList(data: { id: 1, name: "A", todo: { create: { id: 1, name: "B" } } }) {
              id
              name
              todo {
                id
                name
              }
             }
           }"#),
          @r###"{"data":{"createOneList":{"id":1,"name":"A","todo":{"id":1,"name":"B"}}}}"###
        );

        assert_error!(
            runner,
            r#"mutation {
                updateOneList(where: { id: 1 }, data: { name: { set: "C" }, todo: null }) {
                  name
                  todo {
                    id
                    name
                  }
                }
           }"#,
            2009,
            "`Mutation.updateOneList.data.ListUpdateInput.todo`: A value is required but not set."
        );

        Ok(())
    }
}
