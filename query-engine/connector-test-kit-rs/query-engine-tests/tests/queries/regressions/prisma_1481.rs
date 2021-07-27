use query_engine_tests::*;

// validates fix for
// https://github.com/prisma/prisma-engines/issues/1481

#[test_suite(schema(schema), only(Sqlite))]
mod element {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, String, @id)
              email String  @unique
              name  String?
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn prisma_1481(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
        runner.batch(vec![
          r#"mutation {
              executeRaw(
                query: "UPDATE User SET name = ? WHERE id = ?;"
                parameters: "[\"blub1\", \"THIS_DOES_NOT_EXIST1\"]"
              )
            }"#.to_string(),
          r#"mutation {
              updateManyUser(
                where: { name: "A" }
                data:  { name: "B" }
              ) {
                count
              }
            }"#.to_string(),
        ], true).await?.to_string(),
          @r###"{"batchResult":[{"data":{"executeRaw":0}},{"data":{"updateManyUser":{"count":0}}}]}"###
        );

        Ok(())
    }
}
