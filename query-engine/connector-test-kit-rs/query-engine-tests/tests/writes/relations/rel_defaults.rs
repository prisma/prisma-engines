use query_engine_tests::*;

#[test_suite]
mod rel_defaults {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query, run_query_json};

    fn schema_1() -> String {
        let schema = indoc! {
            r#" model List {
              #id(id, Int, @id @default(autoincrement()))
              name  String? @unique
              todoId Int @default(1)
              todo  Todo   @relation(fields: [todoId], references: [id])
            }

            model Todo{
              #id(id, Int, @id @default(autoincrement()))
              name String?
              lists  List[]
            }"#
        };

        schema.to_owned()
    }

    // "Not providing a value for a required relation field with a default value" should "work"
    #[connector_test(schema(schema_1), exclude(MongoDb))]
    async fn no_val_for_required_relation(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ name: "A", todo: { create: { name: "B" } } }"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyList {
              name
              todo {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyList":[{"name":"A","todo":{"name":"B"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTodo { name } }"#),
          @r###"{"data":{"findManyTodo":[{"name":"B"}]}}"###
        );

        assert_eq!(count_items(runner, "findManyList").await?, 1);
        assert_eq!(count_items(runner, "findManyTodo").await?, 1);

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneList(data: { name: "listWithTodoOne" }) {
              id
              todo {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneList":{"id":2,"todo":{"id":1}}}}"###
        );

        assert_eq!(count_items(runner, "findManyList").await?, 2);

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#" model List {
              #id(id, Int, @id @default(autoincrement()))
              name     String? @unique
              todoId    Int @default(1)
              todoName  String
              todo      Todo   @relation(fields: [todoId, todoName], references: [id, name])
           }

           model Todo {
              id Int @default(autoincrement())
              name  String
              lists  List[]

              @@id([id, name])
           }"#
        };

        schema.to_owned()
    }

    // "Not providing a value for a required relation with multiple fields with one default value" should "not work"
    // We ignore SQLite because a multi-column primary key cannot have an autoincrement column on SQLite.
    #[connector_test(schema(schema_2), exclude(MongoDb, Sqlite))]
    async fn no_val_required_rel_one_default_val(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ name: "A", todo: { create: { name: "B"}}}"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyList { name, todo { name } } }"#),
          @r###"{"data":{"findManyList":[{"name":"A","todo":{"name":"B"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTodo { name } }"#),
          @r###"{"data":{"findManyTodo":[{"name":"B"}]}}"###
        );

        assert_eq!(count_items(runner, "findManyList").await?, 1);
        assert_eq!(count_items(runner, "findManyTodo").await?, 1);

        assert_error!(
            runner,
            r#"mutation { createOneList(data: { name: "listWithTodoOne" }) { id todo { id } } }"#,
            2009,
            "`Mutation.createOneList.data.ListCreateInput.todo`: A value is required but not set."
        );

        Ok(())
    }

    // "Not providing a value for one field with a default in a required relation with multiple fields" should "work"
    // We ignore SQLite because a multi-column primary key cannot have an autoincrement column on SQLite.
    // TODO(dom): Mongo not working (@@id)
    #[connector_test(schema(schema_2), exclude(MongoDb, Sqlite))]
    async fn no_val_required_rel_multiple_fields(runner: &Runner) -> TestResult<()> {
        // Test that we can still create with the value without default only
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneList(
              data: { name: "listWithTodoOne", todo: { create: { name: "abcd" } } }
            ) {
              id
              todo {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneList":{"id":1,"todo":{"id":1}}}}"###
        );

        assert_eq!(count_items(runner, "findManyList").await?, 1);
        assert_eq!(count_items(runner, "findManyTodo").await?, 1);

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#" model List {
              #id(id, Int, @id)
              name    String? @unique
              todoId   Int     @default(1)
              todoName String  @default("theTodo")
              todo     Todo    @relation(fields: [todoId, todoName], references: [id, name])
            }

            model Todo{
              id Int @default(1)
              name   String
              lists  List[]
              @@id([id, name])
            }"#
        };

        schema.to_owned()
    }

    // "Not providing a value for required relation fields with default values" should "work"
    // TODO(dom): Not working on mongo. No compound id (yet)?
    #[connector_test(schema(schema_3), exclude(MongoDb))]
    async fn no_val_required_rel_default_vals(runner: &Runner) -> TestResult<()> {
        // Setup
        create_row(runner, r#"{ id: 1, name: "A", todo: { create: { name: "theTodo" } } }"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#" query {
            findManyList {
              name
              todo {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyList":[{"name":"A","todo":{"name":"theTodo"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTodo { name } }"#),
          @r###"{"data":{"findManyTodo":[{"name":"theTodo"}]}}"###
        );

        assert_eq!(count_items(runner, "findManyList").await?, 1);
        assert_eq!(count_items(runner, "findManyTodo").await?, 1);

        insta::assert_snapshot!(
          run_query!(runner, r#" mutation {
            createOneList(data: { id: 2, name: "listWithTheTodo" }) {
              id
              todo {
                id
                name
              }
            }
          }"#),
          @r###"{"data":{"createOneList":{"id":2,"todo":{"id":1,"name":"theTodo"}}}}"###
        );

        assert_eq!(count_items(runner, "findManyList").await?, 2);

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
