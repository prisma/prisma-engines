use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(ScalarLists, DecimalType))]
mod decimal {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
                #id(id, Int, @id)
                decimals Decimal[]
            }"#
        };

        schema.to_owned()
    }

    // "Scalar lists" should "be behave like regular values for create and update operations"
    // Skipped for CockroachDB, lools like this is concat is also broken.
    #[connector_test(exclude(CockroachDb))]
    async fn behave_like_regular_val_for_create_and_update(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation {{
            createOneScalarModel(data: {{
              id: 1,
              decimals:  {{ set: ["1.234", "1.45"] }}
            }}) {{
              decimals
            }}
          }}"#, )),
          @r###"{"data":{"createOneScalarModel":{"decimals":["1.234","1.45"]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              decimals:  { set: ["1.2345678"] }
            }) {
              decimals
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"decimals":["1.2345678"]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              decimals:  { push: "2" }
            }) {
              decimals
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"decimals":["1.2345678","2"]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              decimals:  { push: ["3", "4"] }
            }) {
              decimals
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"decimals":["1.2345678","2","3","4"]}}}"###
        );

        Ok(())
    }

    // "A Create Mutation" should "create and return items with list values with shorthand notation"
    #[connector_test]
    async fn create_mut_work_with_list_vals(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data: {
              id: 1
              decimals: ["1.234", "1.45"]
            }) {
              decimals
            }
          }"#),
          @r###"{"data":{"createOneScalarModel":{"decimals":["1.234","1.45"]}}}"###
        );

        Ok(())
    }

    // "A Create Mutation" should "create and return items with empty list values"
    #[connector_test]
    async fn create_mut_return_items_with_empty_lists(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data: {
              id: 1
              decimals: []
            }) {
              decimals
            }
          }"#),
          @r###"{"data":{"createOneScalarModel":{"decimals":[]}}}"###
        );

        Ok(())
    }

    // "An Update Mutation that pushes to some empty scalar lists" should "work"
    // Skipped for CockroachDB as enum array concatenation is not supported (https://github.com/cockroachdb/cockroach/issues/71388).
    #[connector_test(exclude(CockroachDb))]
    async fn update_mut_push_empty_scalar_list(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2 }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              decimals:  { push: "2" }
            }) {
              decimals
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"decimals":["2"]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 2 }, data: {
              decimals:  { push: ["1", "2"] }
            }) {
              decimals
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"decimals":["1","2"]}}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneScalarModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
