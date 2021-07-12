use query_engine_tests::*;

#[test_suite(schema(schema))]
mod rel_design {
    use indoc::indoc;
    use query_engine_tests::{run_query, run_query_json};

    fn schema() -> String {
        let schema = indoc! {
            r#"model List{
              #id(id, Int, @id)
              uList  String? @unique
              todoId Int?

              todo  Todo?   @relation(fields: [todoId], references: [id])
           }

           model Todo{
              #id(id, Int, @id)
              uTodo String? @unique
              list  List?
           }"#
        };

        schema.to_owned()
    }

    // "Deleting a parent node" should "remove it from the relation and delete the relay id"
    #[connector_test]
    async fn delete_parent_model(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{id: 1, uList: "A", todo : { create: {id: 1, uTodo: "B"}}}"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyList {uList, todo {uTodo}}}"#),
          @r###"{"data":{"findManyList":[{"uList":"A","todo":{"uTodo":"B"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyTodo {uTodo}}"#),
          @r###"{"data":{"findManyTodo":[{"uTodo":"B"}]}}"###
        );

        assert_eq!(count_items(runner, "findManyList").await?, 1);
        assert_eq!(count_items(runner, "findManyTodo").await?, 1);

        run_query!(runner, r#"mutation{deleteOneList(where: {uList:"A"}){id}}"#);

        assert_eq!(count_items(runner, "findManyList").await?, 0);

        Ok(())
    }

    // "Deleting a child node" should "remove it from the relation and delete the relay id"
    #[connector_test]
    async fn delete_child_node(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{id: 1, uList: "A", todo : { create: {id: 1, uTodo: "B"}}}"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyList {uList, todo {uTodo}}}"#),
          @r###"{"data":{"findManyList":[{"uList":"A","todo":{"uTodo":"B"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query{findManyTodo {uTodo}}"#),
          @r###"{"data":{"findManyTodo":[{"uTodo":"B"}]}}"###
        );

        assert_eq!(count_items(runner, "findManyList").await?, 1);
        assert_eq!(count_items(runner, "findManyTodo").await?, 1);

        run_query!(runner, r#"mutation{deleteOneTodo(where: {uTodo:"B"}){id}}"#);

        assert_eq!(count_items(runner, "findManyTodo").await?, 0);

        Ok(())
    }

    async fn count_items(runner: &Runner, name: &str) -> TestResult<usize> {
        let res = run_query_json!(runner, format!(r#"query {{ {} {{ id }} }}"#, name));
        let data = &res["data"][name];

        match data {
            serde_json::Value::Array(arr) => Ok(arr.len()),
            _ => unreachable!(),
        }
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneList(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
