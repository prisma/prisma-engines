use query_engine_tests::*;

#[test_suite(schema(schema))]
mod delete {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
                #id(id, Int, @id)
                string  String?
                unicorn String? @unique
              }"#
        };

        schema.to_owned()
    }

    // "A Delete Mutation" should "delete and return item"
    #[connector_test]
    async fn should_delete_and_return_item(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, string: "test" }"#).await?;

        run_query!(&runner, r#"mutation { deleteOneScalarModel(where: {id: 1}) { id } }"#);

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyScalarModel { id }}"#),
          @r###"{"data":{"findManyScalarModel":[]}}"###
        );

        Ok(())
    }

    // "A Delete Mutation" should "gracefully fail on non-existing id"
    #[connector_test]
    async fn should_fail_non_exist_id(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, string: "test" }"#).await?;

        assert_error!(
          runner,
          r#"mutation { deleteOneScalarModel(where: {id: 2 }){ id }}"#,
          2025,
          "An operation failed because it depends on one or more records that were required but not found. No record was found for a delete."
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyScalarModel { string }}"#),
          @r###"{"data":{"findManyScalarModel":[{"string":"test"}]}}"###
        );

        Ok(())
    }

    // "A Delete Mutation" should "delete and return item on non id unique field"
    #[connector_test]
    async fn should_delete_return_non_id_uniq_field(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, unicorn: "a" }"#).await?;
        create_row(&runner, r#"{ id: 2, unicorn: "b" }"#).await?;

        run_query!(
            &runner,
            r#"mutation { deleteOneScalarModel(where: { unicorn: "a" }) { unicorn }}"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyScalarModel{ unicorn } }"#),
          @r###"{"data":{"findManyScalarModel":[{"unicorn":"b"}]}}"###
        );

        Ok(())
    }

    // "A Delete Mutation" should "gracefully fail when trying to delete on non-existent value for non id unique field"
    #[connector_test]
    async fn should_fail_non_existent_value_non_id_uniq_field(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{id: 1, unicorn: "a"}"#).await?;

        assert_error!(
          runner,
          r#"mutation { deleteOneScalarModel(where: {unicorn: "c"}) { unicorn }}"#,
          2025,
          "An operation failed because it depends on one or more records that were required but not found. No record was found for a delete."
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyScalarModel { unicorn } }"#),
          @r###"{"data":{"findManyScalarModel":[{"unicorn":"a"}]}}"###
        );

        Ok(())
    }

    // "A Delete Mutation" should "gracefully fail when trying to delete on null value for unique field"
    #[connector_test]
    async fn should_fail_delete_null_value(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{id: 1, unicorn: "a"}"#).await?;

        assert_error!(
            &runner,
            r#"mutation { deleteOneScalarModel(where: {unicorn: null}) { unicorn }}"#,
            2012,
            "A value is required but not set"
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyScalarModel { unicorn }}"#),
          @r###"{"data":{"findManyScalarModel":[{"unicorn":"a"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(generic))]
    async fn delete_fails_if_filter_dont_match(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: { id: 1, field: "hello" }) { id } }"#
        );

        assert_error!(
            &runner,
            r#"mutation {
                  deleteOneTestModel(where: { id: 1, field: "bonjour" }) {
                    id
                  }
                }"#,
            2025,
            "An operation failed because it depends on one or more records that were required but not found. No record was found for a delete."
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneScalarModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
