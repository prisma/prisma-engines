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

        let uuid = res["data"]["createOneTodo"]["id"]
            .as_str()
            .expect("Expected string ID but got something else.");

        uuid::Uuid::parse_str(uuid).expect("Expected valid UUID but couldn't parse it.");

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

        let uuid = res["data"]["createOneTodo"]["id"]
            .as_str()
            .expect("Expected string ID but got something else.");

        // Validate that this is a valid UUIDv7 value
        {
            let uuid = uuid::Uuid::parse_str(uuid).expect("Expected valid UUID but couldn't parse it.");
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
        let uuid_find_many = res["data"]["findManyTodo"][0]["id"]
            .as_str()
            .expect("Expected string ID but got something else.");
        assert_eq!(uuid_find_many, uuid);

        // Test findUnique
        let res = run_query_json!(
            &runner,
            format!(r#"query {{ findUniqueTodo(where: {{ id: "{}" }}) {{ id }} }}"#, uuid)
        );
        let uuid_find_unique = res["data"]["findUniqueTodo"]["id"]
            .as_str()
            .expect("Expected string ID but got something else.");
        assert_eq!(uuid_find_unique, uuid);

        Ok(())
    }

    fn schema_cuid_1() -> String {
        let schema = indoc! {
            r#"model Todo {
                #id(id, String, @id, @default(cuid(1)))
                title String
            }"#
        };

        schema.to_owned()
    }

    // "Creating an item with an id field of model CUIDv1 and retrieving it" should "work"
    #[connector_test(schema(schema_cuid_1))]
    async fn create_cuid_v1_and_retrieve_it_should_work(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {
                createOneTodo(data: { title: "the title" }){
                    id
                }
            }"#
        );

        let cuid_1: &str = res["data"]["createOneTodo"]["id"]
            .as_str()
            .expect("Expected string ID but got something else.");

        // Validate that this is a valid CUIDv1 value
        assert!(cuid::is_cuid1(cuid_1));

        // Test findMany
        let res = run_query_json!(
            &runner,
            r#"query { findManyTodo(where: { title: "the title" }) { id }}"#
        );
        let cuid_find_many = res["data"]["findManyTodo"][0]["id"]
            .as_str()
            .expect("Expected string ID but got something else.");
        assert_eq!(cuid_find_many, cuid_1);

        // Test findUnique
        let res = run_query_json!(
            &runner,
            format!(r#"query {{ findUniqueTodo(where: {{ id: "{}" }}) {{ id }} }}"#, cuid_1)
        );
        let uuid_find_unique = res["data"]["findUniqueTodo"]["id"]
            .as_str()
            .expect("Expected string ID but got something else.");
        assert_eq!(uuid_find_unique, cuid_1);

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

        let cuid_2 = res["data"]["createOneTodo"]["id"]
            .as_str()
            .expect("Expected string ID but got something else.");

        // Validate that this is a valid CUIDv2 value
        assert!(cuid::is_cuid2(cuid_2));

        // Test findMany
        let res = run_query_json!(
            &runner,
            r#"query { findManyTodo(where: { title: "the title" }) { id }}"#
        );
        let cuid_find_many = res["data"]["findManyTodo"][0]["id"]
            .as_str()
            .expect("Expected string ID but got something else.");
        assert_eq!(cuid_find_many, cuid_2);

        // Test findUnique
        let res = run_query_json!(
            &runner,
            format!(r#"query {{ findUniqueTodo(where: {{ id: "{}" }}) {{ id }} }}"#, cuid_2)
        );
        let cuid_find_unique = res["data"]["findUniqueTodo"]["id"]
            .as_str()
            .expect("Expected string ID but got something else.");
        assert_eq!(cuid_find_unique, cuid_2);

        Ok(())
    }
}
