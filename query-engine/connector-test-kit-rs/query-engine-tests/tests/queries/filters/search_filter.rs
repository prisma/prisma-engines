use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(FullTextSearchWithoutIndex))]
mod search_filter {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              fieldA  String
              fieldB  String
              fieldC  String?
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn search_single_field(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { fieldA: { search: "Chicken" } }) { fieldA } }"#),
          @r###"{"data":{"findManyTestModel":[{"fieldA":"Chicken Masala"},{"fieldA":"Chicken Curry"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn search_many_fields(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: {
                  fieldA: { search: "Chicken" }
                  fieldB: { search: "Chicken" }
              }) { fieldA, fieldB }}
        "#),
          @r###"{"data":{"findManyTestModel":[{"fieldA":"Chicken Masala","fieldB":"Chicken, Rice, Masala Sauce"},{"fieldA":"Chicken Curry","fieldB":"Chicken, Curry"},{"fieldA":"Caesar Salad","fieldB":"Salad, Chicken, Caesar Sauce"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn search_nullable_field(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: {
                    fieldA: { search: "Chicken" }
                    fieldC: { search: "Chicken" }
                }) { fieldA, fieldC }}
          "#),
          @r###"{"data":{"findManyTestModel":[{"fieldA":"Caesar Salad","fieldC":"Chicken"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn search_with_other_filters(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: {
                    fieldA: { search: "Chicken", startsWith: "Chicken" },
                    fieldB: { search: "Chicken" },
                    id: { equals: 1 }
                }) { fieldA, fieldB, fieldC }}
          "#),
          @r###"{"data":{"findManyTestModel":[{"fieldA":"Chicken Masala","fieldB":"Chicken, Rice, Masala Sauce","fieldC":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn ensure_filter_tree_shake_works(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: {
                    AND: [
                        { fieldA: { search: "Chicken", startsWith: "Chicken" } },
                        { OR: [{ fieldB: { search: "Chicken" } }, { id: { equals: 3 } }] }
                    ]
                }) { id, fieldA, fieldB, fieldC }}
          "#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"fieldA":"Chicken Masala","fieldB":"Chicken, Rice, Masala Sauce","fieldC":null},{"id":2,"fieldA":"Chicken Curry","fieldB":"Chicken, Curry","fieldC":null}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, fieldA: "Chicken Masala", fieldB: "Chicken, Rice, Masala Sauce"}"#,
        )
        .await?;
        create_row(runner, r#"{ id: 2, fieldA: "Chicken Curry", fieldB: "Chicken, Curry"}"#).await?;
        create_row(
            runner,
            r#"{ id: 3, fieldA: "Caesar Salad", fieldB: "Salad, Chicken, Caesar Sauce", fieldC: "Chicken"}"#,
        )
        .await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
