use query_engine_tests::*;

#[test_suite(schema(schema))]
mod views {
    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              firstName String
              lastName  String
            }

            view TestView {
              #id(id, Int, @id)

              firstName String
              lastName  String
              fullName  String
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn no_create_one_mutation(runner: Runner) -> TestResult<()> {
        test_no_toplevel_mutation(
            runner,
            r#"mutation { createOneTestView(data: { firstName: "Test", lastName: "User", fullName: "Test User" }) { id } }"#
        ).await
    }

    #[connector_test]
    async fn no_update_one_mutation(runner: Runner) -> TestResult<()> {
        test_no_toplevel_mutation(
            runner,
            r#"mutation { updateOneTestView(where: { id: 1 }, data: { firstName: "Updated" }) { id } }"#,
        )
        .await
    }

    #[connector_test]
    async fn no_delete_one_mutation(runner: Runner) -> TestResult<()> {
        test_no_toplevel_mutation(runner, r#"mutation { deleteOneTestView(where: { id: 1 }) { id } }"#).await
    }

    #[connector_test]
    async fn no_upsert_one_mutation(runner: Runner) -> TestResult<()> {
        test_no_toplevel_mutation(
            runner,
            r#"mutation { upsertOneTestView(where: { id: 1 }, create: { firstName: "New", lastName: "User", fullName: "New User" }, update: { firstName: "Updated" }) { id } }"#
        ).await
    }

    #[connector_test]
    async fn no_create_many_mutation(runner: Runner) -> TestResult<()> {
        test_no_toplevel_mutation(
            runner,
            r#"mutation { createManyTestView(data: [{ firstName: "Test", lastName: "User", fullName: "Test User" }]) { count } }"#
        ).await
    }

    #[connector_test]
    async fn no_update_many_mutation(runner: Runner) -> TestResult<()> {
        test_no_toplevel_mutation(
            runner,
            r#"mutation { updateManyTestView(where: { id: 1 }, data: { firstName: "Updated" }) { count } }"#,
        )
        .await
    }

    #[connector_test]
    async fn no_delete_many_mutation(runner: Runner) -> TestResult<()> {
        test_no_toplevel_mutation(
            runner,
            r#"mutation { deleteManyTestView(where: { id: { gt: 0 } }) { count } }"#,
        )
        .await
    }

    async fn test_no_toplevel_mutation(runner: Runner, query: &str) -> TestResult<()> {
        match runner.query(query).await {
            Ok(res) => res.assert_failure(2009, None),
            Err(TestError::QueryConversionError(err)) if err.kind().code() == "P2009" => (),
            Err(err) => return Err(err),
        }

        Ok(())
    }
}
