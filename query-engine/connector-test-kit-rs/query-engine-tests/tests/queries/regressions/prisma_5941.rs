use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(AnyId))]
mod regression {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Artist {
                firstName String
                lastName  String
                birth     DateTime

                @@unique([firstName, lastName, birth])
              }"#
        };

        schema.to_owned()
    }

    // "input dates in two queries" should "not return nulls"
    #[connector_test]
    async fn input_dates_no_nulls(runner: Runner) -> TestResult<()> {
        runner
            .query(indoc! {
                r#"mutation {createOneArtist(data:{
                    firstName: "Sponge"
                    lastName: "Bob"
                    birth: "1999-05-01T00:00:00.000Z"
                }){ firstName lastName birth }}"#
            })
            .await?
            .assert_success();

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName_birth:{firstName:"Sponge",lastName:"Bob", birth: "1999-05-01T00:00:00.000Z"}}) {firstName lastName birth}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName_birth:{firstName:"Sponge",lastName:"Bob", birth: "1999-05-01T00:00:00.000Z"}}) {firstName lastName birth}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Sponge","lastName":"Bob","birth":"1999-05-01T00:00:00.000Z"}}},{"data":{"findUniqueArtist":{"firstName":"Sponge","lastName":"Bob","birth":"1999-05-01T00:00:00.000Z"}}}]}"###
        );

        Ok(())
    }
}
