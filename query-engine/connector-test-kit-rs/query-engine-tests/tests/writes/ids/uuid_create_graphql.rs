use query_engine_tests::*;

#[test_suite(schema(schema))]
mod uuid_create_graphql {
    use indoc::indoc;
    use query_engine_tests::{run_query, run_query_json};

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, String, @id, @default(uuid()))
              title String
            }"#
        };

        schema.to_owned()
    }

    // "Creating an item with an id field of model UUID" should "work"
    #[connector_test(schema(schema_1))]
    async fn create_uuid_id_should_work(runner: &Runner) -> TestResult<()> {
        let res = run_query_json!(
            runner,
            r#"mutation {
          createOneTodo(data: { title: "the title" }){
            id
            title
          }
        }"#
        );

        insta::assert_snapshot!(
          &res["data"]["createOneTodo"]["title"].to_string(),
          @r###""the title""###
        );

        let uuid = match &res["data"]["createOneTodo"]["id"] {
            serde_json::Value::String(str) => str,
            _ => unreachable!(),
        };

        uuid::Uuid::parse_str(uuid.as_str()).expect("Expected valid UUID but couldn't parse it.");

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model TableA {
              #id(id, String, @id, @default(uuid()))
              name  String
              b     String? @unique
          }"#
        };

        schema.to_owned()
    }

    // "Fetching a UUID field that is null" should "work"
    #[connector_test(schema(schema_2))]
    async fn fetch_null_uuid_should_work(runner: &Runner) -> TestResult<()> {
        run_query!(runner, r#"mutation {createOneTableA(data: {name:"testA"}){id}}"#);

        insta::assert_snapshot!(
          run_query!(runner, r#"{findManyTableA {name, b}}"#),
          @r###"{"data":{"findManyTableA":[{"name":"testA","b":null}]}}"###
        );

        Ok(())
    }
}
