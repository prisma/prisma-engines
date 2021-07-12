use query_engine_tests::*;

#[test_suite(schema(schema))]
mod where_and_update {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Test {
              #id(id, Int, @id)
              unique  Int    @unique
              name    String
           }"#
        };

        schema.to_owned()
    }

    // "Updating the unique value used to find an item" should "work"
    #[connector_test]
    async fn update_unique_val(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{id: 1, unique: 1, name: "Test"}"#).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {findUniqueTest(where:{unique:1}){ unique }}"#),
          @r###"{"data":{"findUniqueTest":{"unique":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {updateOneTest( where: { unique: 1 } data: { unique: { set: 2 }}){unique}}"#),
          @r###"{"data":{"updateOneTest":{"unique":2}}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTest(data: {}) {{ unique }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
