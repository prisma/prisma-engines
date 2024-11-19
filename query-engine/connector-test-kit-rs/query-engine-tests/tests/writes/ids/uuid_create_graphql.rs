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
    async fn create_uuid_id_should_work(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
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
    async fn fetch_null_uuid_should_work(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation {createOneTableA(data: {name:"testA"}){id}}"#);

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyTableA {name, b}}"#),
          @r###"{"data":{"findManyTableA":[{"name":"testA","b":null}]}}"###
        );

        Ok(())
    }

    fn schema_uuid_7() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, String, @id, @default(uuid(7)))
              title String
            }"#
        };

        schema.to_owned()
    }

    // "Creating an item with an id field of model UUIDv7 and retrieving it" should "work"
    #[connector_test(schema(schema_uuid_7))]
    async fn create_uuid_v7_and_retrieve_it_should_work(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {
          createOneTodo(data: { title: "the title" }){
            id
          }
        }"#
        );

        let uuid = match &res["data"]["createOneTodo"]["id"] {
            serde_json::Value::String(str) => str,
            _ => unreachable!(),
        };

        // Validate that this is a valid UUIDv7 value
        {
            let uuid = uuid::Uuid::parse_str(uuid.as_str()).expect("Expected valid UUID but couldn't parse it.");
            assert_eq!(
                uuid.get_version().expect("Expected UUIDv7 but got something else."),
                uuid::Version::SortRand
            );
        }

        // Test findMany
        let res = run_query_json!(
            &runner,
            r#"query { findManyTodo(where: { title: "the title" }) { id }}"#
        );
        if let serde_json::Value::String(str) = &res["data"]["findManyTodo"][0]["id"] {
            assert_eq!(str, uuid);
        } else {
            panic!("Expected UUID but got something else.");
        }

        // Test findUnique
        let res = run_query_json!(
            &runner,
            format!(r#"query {{ findUniqueTodo(where: {{ id: "{}" }}) {{ id }} }}"#, uuid)
        );
        if let serde_json::Value::String(str) = &res["data"]["findUniqueTodo"]["id"] {
            assert_eq!(str, uuid);
        } else {
            panic!("Expected UUID but got something else.");
        }

        Ok(())
    }

    fn schema_cuid_2() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, String, @id, @default(cuid(2)))
              title String
            }"#
        };

        schema.to_owned()
    }

    // "Creating an item with an id field of model CUIDv2 and retrieving it" should "work"
    #[connector_test(schema(schema_cuid_2))]
    async fn create_cuid_v2_and_retrieve_it_should_work(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {
          createOneTodo(data: { title: "the title" }){
            id
          }
        }"#
        );

        let cuid = match &res["data"]["createOneTodo"]["id"] {
            serde_json::Value::String(str) => str,
            _ => unreachable!(),
        };

        // Validate that this is a valid CUIDv2 value
        {
            assert!(cuid::is_cuid2(cuid.as_str()));
        }

        // Test findMany
        let res = run_query_json!(
            &runner,
            r#"query { findManyTodo(where: { title: "the title" }) { id }}"#
        );
        if let serde_json::Value::String(str) = &res["data"]["findManyTodo"][0]["id"] {
            assert_eq!(str, cuid);
        } else {
            panic!("Expected CUID but got something else.");
        }

        // Test findUnique
        let res = run_query_json!(
            &runner,
            format!(r#"query {{ findUniqueTodo(where: {{ id: "{}" }}) {{ id }} }}"#, cuid)
        );
        if let serde_json::Value::String(str) = &res["data"]["findUniqueTodo"]["id"] {
            assert_eq!(str, cuid);
        } else {
            panic!("Expected CUID but got something else.");
        }

        Ok(())
    }
}
