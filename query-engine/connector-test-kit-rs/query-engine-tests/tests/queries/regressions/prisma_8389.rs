use indoc::indoc;
use query_engine_tests::*;

// Validates fix for
// https://github.com/prisma/prisma/issues/8389

#[test_suite(schema(schema), only(MongoDb))]
mod prisma_8389 {
    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
               }
            "#
        };

        schema.to_owned()
    }

    // Ensures that the MongoDB Rust driver's internal pagination does not fail
    // fetching more than one page. 101 documents is the default batch size.
    // We intentionally create 101+ documents (103, arbitrarily) to assert
    // no errors are raised when querying them
    #[connector_test]
    async fn find_many_more_than_101_should_work(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let res = run_query_json!(
            runner,
            r#"query { findManyTestModel { id } }"#,
            &["data", "findManyTestModel"]
        );

        assert_eq!(res.is_array(), true);

        let res = res.as_array().unwrap();

        assert_eq!(res.len(), 103);

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        let data = (1..104)
            .map(|n| format!("{{ id: {} }}", n))
            .collect::<Vec<String>>()
            .join(", ");

        runner
            .query(format!(
                "mutation {{ createManyTestModel(data: [{}]) {{ count }} }}",
                data
            ))
            .await?
            .assert_success();

        Ok(())
    }
}
