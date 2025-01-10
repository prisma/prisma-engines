use query_engine_tests::*;

#[test_suite(schema(schema))]
mod delete_many {
    use indoc::indoc;
    use query_engine_tests::{run_query, run_query_json};

    fn schema() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, Int, @id)
              title String
            }"#
        };

        schema.to_owned()
    }

    // "The delete many Mutation" should "delete the items matching the where clause"
    #[connector_test]
    async fn should_delete_items(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, title: "title1" }"#).await?;
        create_row(&runner, r#"{ id: 2, title: "title2" }"#).await?;

        assert_todo_count(&runner, 2).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteManyTodo(
              where: { title: { equals: "title1" }}
            ){
              count
            }
          }"#),
          @r###"{"data":{"deleteManyTodo":{"count":1}}}"###
        );

        assert_todo_count(&runner, 1).await?;

        Ok(())
    }

    // "The delete many Mutation" should "delete all items if the where clause is empty"
    #[connector_test]
    async fn should_delete_all_if_where_empty(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, title: "title1" }"#).await?;
        create_row(&runner, r#"{ id: 2, title: "title2" }"#).await?;
        create_row(&runner, r#"{ id: 3, title: "title3" }"#).await?;
        assert_todo_count(&runner, 3).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteManyTodo(where: {}){
              count
            }
          }"#),
          @r###"{"data":{"deleteManyTodo":{"count":3}}}"###
        );

        assert_todo_count(&runner, 0).await?;

        Ok(())
    }

    // "The delete many Mutation" should "delete all items using in"
    #[connector_test]
    async fn should_delete_all_using_in(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, title: "title1" }"#).await?;
        create_row(&runner, r#"{ id: 2, title: "title2" }"#).await?;
        create_row(&runner, r#"{ id: 3, title: "title3" }"#).await?;

        assert_todo_count(&runner, 3).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteManyTodo(where: { title: { in: [ "title1", "title2" ] }}) {
              count
            }
          }"#),
          @r###"{"data":{"deleteManyTodo":{"count":2}}}"###
        );

        assert_todo_count(&runner, 1).await?;

        Ok(())
    }

    // "The delete many Mutation" should "delete all items using notin"
    #[connector_test]
    async fn should_delete_all_using_notin(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, title: "title1" }"#).await?;
        create_row(&runner, r#"{ id: 2, title: "title2" }"#).await?;
        create_row(&runner, r#"{ id: 3, title: "title3" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteManyTodo(
              where: { title: { not: { in: [ "DoesNotExist", "AlsoDoesntExist" ] }}}
            ){
              count
            }
          }"#),
          @r###"{"data":{"deleteManyTodo":{"count":3}}}"###
        );

        assert_todo_count(&runner, 0).await?;

        Ok(())
    }

    // "The delete many Mutation" should "delete items using OR"
    #[connector_test]
    async fn should_delete_using_or(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, title: "title1" }"#).await?;
        create_row(&runner, r#"{ id: 2, title: "title2" }"#).await?;
        create_row(&runner, r#"{ id: 3, title: "title3" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyTodo(
              where: { OR: [{ title: { equals: "title1" } }, { title: { equals: "title2" }}]}
            ) {
              title
            }
          }"#),
          @r###"{"data":{"findManyTodo":[{"title":"title1"},{"title":"title2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteManyTodo(
              where: { OR: [{ title: { equals: "title1" }}, { title: { equals: "title2" }}]}
            ){
              count
            }
          }"#),
          @r###"{"data":{"deleteManyTodo":{"count":2}}}"###
        );

        assert_todo_count(&runner, 1).await?;

        Ok(())
    }

    // "The delete many Mutation" should "delete items using  AND"
    #[connector_test]
    async fn should_delete_using_and(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, title: "title1" }"#).await?;
        create_row(&runner, r#"{ id: 2, title: "title2" }"#).await?;
        create_row(&runner, r#"{ id: 3, title: "title3" }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyTodo(
              where: { AND: [{ title: { equals: "title1" }}, { title: { equals: "title2" }}]}
            ){
              title
            }
          }"#),
          @r###"{"data":{"findManyTodo":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteManyTodo(
              where: { AND: [{ title: { equals: "title1" }}, { title: { equals: "title2" }}]}
            ){
              count
            }
          }"#),
          @r###"{"data":{"deleteManyTodo":{"count":0}}}"###
        );

        assert_todo_count(&runner, 3).await?;

        Ok(())
    }

    // "The delete many Mutation" should "delete max the number of items specified in the limit"
    #[connector_test]
    async fn should_delete_max_limit_items(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, title: "title1" }"#).await?;
        create_row(&runner, r#"{ id: 2, title: "title2" }"#).await?;
        create_row(&runner, r#"{ id: 3, title: "title3" }"#).await?;
        create_row(&runner, r#"{ id: 4, title: "title4" }"#).await?;

        assert_todo_count(&runner, 4).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            deleteManyTodo(
              limit: 3
            ){
              count
            }
          }"#),
          @r###"{"data":{"deleteManyTodo":{"count":3}}}"###
        );

        assert_todo_count(&runner, 1).await?;

        Ok(())
    }

    // "The delete many Mutation" should "fail if limit param is negative"
    #[connector_test]
    async fn should_fail_with_negative_limit(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, title: "title1" }"#).await?;
        create_row(&runner, r#"{ id: 2, title: "title2" }"#).await?;
        create_row(&runner, r#"{ id: 3, title: "title3" }"#).await?;
        create_row(&runner, r#"{ id: 4, title: "title4" }"#).await?;

        assert_error!(
            &runner,
            r#"mutation { deleteManyTodo(limit: -3){ count }}"#,
            2019,
            "Provided limit (-3) must be a positive integer."
        );

        Ok(())
    }

    fn nested_del_many() -> String {
        let schema = indoc! {
            r#"model ZChild{
              #id(id, String, @id)
              name     String? @unique
              test     String?
              parentId String?

              parent Parent? @relation(fields: [parentId], references: [id])
          }

          model Parent{
              #id(id, String, @id)
              name     String? @unique
              children ZChild[]
          }"#
        };

        schema.to_owned()
    }

    // "nested DeleteMany" should "work"
    #[connector_test(schema(nested_del_many))]
    async fn nested_delete_many(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneParent(data: {
              id: "1",
              name: "Dad",
              children: {create:[{ id: "1", name: "Daughter"},{ id: "2", name: "Daughter2"}, { id: "3", name: "Son"}, { id: "4", name: "Son2"}]}
            }){
              name,
              children { name }
            }}"#),
          @r###"{"data":{"createOneParent":{"name":"Dad","children":[{"name":"Daughter"},{"name":"Daughter2"},{"name":"Son"},{"name":"Son2"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneParent(
              where: { name: "Dad" }
              data: { children: { deleteMany: [
                { name: { contains: "Daughter" } },
                { name: { contains: "Son" } }
              ]}
            }){
            name,
            children{ name }
          }}"#),
          @r###"{"data":{"updateOneParent":{"name":"Dad","children":[]}}}"###
        );

        Ok(())
    }

    async fn assert_todo_count(runner: &Runner, count: usize) -> TestResult<()> {
        let res = run_query_json!(runner, r#"{ findManyTodo { id } }"#);

        match &res["data"]["findManyTodo"] {
            serde_json::Value::Array(array_res) => {
                assert_eq!(count, array_res.len());
            }
            _ => {
                panic!("Unexpected result when counting todos: {}", res);
            }
        }

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTodo(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
