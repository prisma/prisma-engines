use query_engine_tests::*;

// exclude: mongodb does not support Decimal
#[test_suite(schema(schema), exclude(MongoDb))]
mod regression {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Artist {
                firstName String
                netWorth  Decimal

                @@unique([firstName, netWorth])
              }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn decimal_find_different_uniques(runner: Runner) -> TestResult<()> {
        runner
            .query(indoc! {
                r#"mutation {createOneArtist(data:{
                    firstName: "Michael"
                    netWorth: "236600000.12409"
                }){ firstName }}"#
            })
            .await?
            .assert_success();

        runner
            .query(indoc! {
                r#"mutation {createOneArtist(data:{
                    firstName: "George"
                    netWorth: "-0.23660010012409"
                }){ firstName }}"#
            })
            .await?
            .assert_success();

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_netWorth:{firstName:"Michael",netWorth:"236600000.12409"}}) {firstName netWorth}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_netWorth:{firstName:"George",netWorth:"-0.23660010012409"}}) {firstName netWorth}}"#.to_string(),
        ];

        let doc = compact_batch(&runner, queries.clone()).await?;
        assert!(doc.is_compact());

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Michael","netWorth":"236600000.12409"}}},{"data":{"findUniqueArtist":{"firstName":"George","netWorth":"-0.23660010012409"}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn decimal_find_different_uniques_unquoted(runner: Runner) -> TestResult<()> {
        runner
            .query(indoc! {
                r#"mutation {createOneArtist(data:{
                    firstName: "Michael"
                    netWorth: 236600000.12409
                }){ firstName }}"#
            })
            .await?
            .assert_success();

        runner
            .query(indoc! {
                r#"mutation {createOneArtist(data:{
                    firstName: "George"
                    netWorth: -0.23660010012409
                }){ firstName }}"#
            })
            .await?
            .assert_success();

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_netWorth:{firstName:"Michael",netWorth:236600000.12409}}) {firstName netWorth}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_netWorth:{firstName:"George",netWorth:-0.23660010012409}}) {firstName netWorth}}"#.to_string(),
        ];

        let doc = compact_batch(&runner, queries.clone()).await?;
        assert!(doc.is_compact());

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Michael","netWorth":"236600000.12409"}}},{"data":{"findUniqueArtist":{"firstName":"George","netWorth":"-0.23660010012409"}}}]}"###
        );

        Ok(())
    }
}
