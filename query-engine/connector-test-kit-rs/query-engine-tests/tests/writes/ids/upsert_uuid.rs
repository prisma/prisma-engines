use query_engine_tests::*;

#[test_suite]
mod upsert_uuid {
    use indoc::indoc;
    use query_engine_tests::run_query_json;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, String, @id, @default(uuid()))
              title  String
            }"#
        };

        schema.to_owned()
    }

    // "Upserting an item with an id field of model UUID" should "work"
    #[connector_test(schema(schema))]
    async fn upsert_id_uuid_should_work(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {
                upsertOneTodo(
                  where: {id: "00000000-0000-0000-0000-000000000000"}
                  create: { title: "the title" }
                  update: { title: { set: "the updated title" } }
                ){
                  id
                  title
                }
            }"#
        );

        insta::assert_snapshot!(
          &res["data"]["upsertOneTodo"]["title"].to_string(),
          @r###""the title""###
        );

        let uuid = match &res["data"]["upsertOneTodo"]["id"] {
            serde_json::Value::String(str) => str,
            _ => unreachable!(),
        };

        uuid::Uuid::parse_str(uuid.as_str()).expect("Expected valid UUID but couldn't parse it.");

        Ok(())
    }
}
