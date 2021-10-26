use query_engine_tests::*;

#[test_suite(schema(dummy))]
mod query_raw {

    fn dummy() -> String {
        let schema = indoc! {r#"
        model Test {
            id  Int @id
        }
        "#};

        schema.to_string()
    }

    // MariaDB is the only one supporting anon blocks
    #[connector_test(only(MySQL("mariadb")))]
    async fn mysql_call(runner: Runner) -> TestResult<()> {
        // fmt_execute_raw cannot run this query, doing it directly instead
        runner
            .query(indoc! {r#"
            mutation {
                queryRaw(
                    query: "BEGIN NOT ATOMIC\n INSERT INTO Test VALUES(FLOOR(RAND()*1000));\n SELECT * FROM Test;\n END",
                    parameters: "[]"
                )
            }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}
