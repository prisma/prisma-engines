use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(AnyId))]
mod compound_batch {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Artist {
                firstName String
                lastName  String

                @@unique([firstName, lastName])
              }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn one_success(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query { findUniqueArtist(where: { firstName_lastName: { firstName:"Musti", lastName:"Naukio" }}) { firstName lastName }}"#.to_string()
        ];

        let batch_results = runner.batch(queries, false).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn two_success_one_fail(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}}) {firstName lastName}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn two_success_sel_set_reorder(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}}) {lastName firstName}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"###
        );

        Ok(())
    }

    // "Two successful queries and one failing with different selection set" should "work"
    #[connector_test]
    async fn two_success_one_fail_diff_set(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
           r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}"#.to_string(),
           r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {lastName}}"#.to_string(),
           r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}}) {firstName lastName}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {lastName}}"#
                .to_string(),
        ];

        let batch_results = runner.batch(queries, false).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":null}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one_failure_one_success(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {firstName lastName}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}}]}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneArtist(data: { firstName: "Musti" lastName: "Naukio" }) { firstName }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneArtist(data: { firstName: "Naukio" lastName: "Musti" }) { firstName }}"#)
            .await?
            .assert_success();

        Ok(())
    }
}
