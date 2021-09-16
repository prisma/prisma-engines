use query_engine_tests::*;

#[test_suite(schema(schema))]
mod datetime {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Person {
              #id(id, Int, @id)
              name String   @unique
              born DateTime
             }"#
        };

        schema.to_owned()
    }

    // "Using a date before 1970" should "work"
    // FIXME: this panics the rust code. Let's fix that at some point.
    #[connector_test(exclude(Sqlite))]
    async fn before_1970(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{id: 1, name: "First", born: "1969-01-01T10:33:59Z"}"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {findUniquePerson(where:{name: "First"}){name, born}}"#),
          @r###"{"data":{"findUniquePerson":{"name":"First","born":"1969-01-01T10:33:59.000Z"}}}"###
        );

        Ok(())
    }

    // "Using milliseconds in a date before 1970" should "work"
    #[connector_test(exclude(Sqlite))]
    async fn ms_in_date_before_1970(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{id: 1, name: "Second", born: "1969-01-01T10:33:59.828Z"}"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {findUniquePerson(where:{name: "Second"}){name, born}}"#),
          @r###"{"data":{"findUniquePerson":{"name":"Second","born":"1969-01-01T10:33:59.828Z"}}}"###
        );

        Ok(())
    }

    // "Using a date after 1970" should "work"
    #[connector_test]
    async fn date_after_1970(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{id: 1, name: "Third", born: "1979-01-01T10:33:59Z"}"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {findUniquePerson(where:{name: "Third"}){name, born}}"#),
          @r###"{"data":{"findUniquePerson":{"name":"Third","born":"1979-01-01T10:33:59.000Z"}}}"###
        );

        Ok(())
    }

    // "Using milliseconds in a date after 1970" should "work"
    #[connector_test]
    async fn ms_in_date_after_1970(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{id: 1, name: "Fourth", born: "1979-01-01T10:33:59.828Z"}"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {findUniquePerson(where:{name: "Fourth"}){name, born}}"#),
          @r###"{"data":{"findUniquePerson":{"name":"Fourth","born":"1979-01-01T10:33:59.828Z"}}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOnePerson(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
