use query_engine_tests::*;

#[test_suite]
mod nanoid {
    use indoc::indoc;

    fn schema_nanoid() -> String {
        let schema = indoc! {
            r#"model Todo {
                #id(id, String, @id, @default(nanoid()))
                title String
            }"#
        };

        schema.to_owned()
    }

    // "Creating an item with an id field of type String with nanoid+length default"
    #[connector_test(schema(schema_nanoid))]
    async fn create_base_nanoid_id_should_work(runner: Runner) -> TestResult<()> {
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

        let nanoid = match &res["data"]["createOneTodo"]["id"] {
            serde_json::Value::String(str) => str,
            _ => unreachable!(),
        };

        assert_eq!(nanoid.len(), 21);

        Ok(())
    }

    fn schema_nanoid_length() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, String, @id, @default(nanoid(7)))
              title String
            }"#
        };

        schema.to_owned()
    }

    // "Creating an item with an id field of type String with nanoid+length default"
    #[connector_test(schema(schema_nanoid_length))]
    async fn create_nanoid_id_with_length_should_work(runner: Runner) -> TestResult<()> {
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

        let nanoid = match &res["data"]["createOneTodo"]["id"] {
            serde_json::Value::String(str) => str,
            _ => unreachable!(),
        };

        assert_eq!(nanoid.len(), 7);

        Ok(())
    }
}
