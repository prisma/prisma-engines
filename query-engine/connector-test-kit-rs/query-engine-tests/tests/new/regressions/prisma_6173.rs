use query_engine_tests::*;

#[test_suite(schema(empty))]
mod query_raw {

    // MariaDB is the only one supporting anon blocks
    #[connector_test(only(MySQL("mariadb")))]
    async fn mysql_call(runner: Runner) -> TestResult<()> {
        // Create a simple table for the test

        runner
            .query(fmt_execute_raw("CREATE TABLE test (id INT PRIMARY KEY);", vec![]))
            .await?
            .assert_success();

        // fmt_execute_raw cannot run this query, doing it directly instead
        runner
            .query(indoc! {r#"
            mutation {
                queryRaw(
                    query: "BEGIN NOT ATOMIC\n INSERT INTO test VALUES(FLOOR(RAND()*1000));\n SELECT * FROM test;\n END",
                    parameters: "[]"
                )
            }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}
