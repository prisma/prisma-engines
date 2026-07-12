use query_engine_tests::*;

// create a set of common tests to use across connectors with full text indexes or without.

async fn search_single_field(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    insta::assert_snapshot!(
      run_query!(&runner, r#"query { findManyTestModel(where: { fieldA: { search: "Chicken" } }) { fieldA } }"#),
      @r###"{"data":{"findManyTestModel":[{"fieldA":"Chicken Masala"},{"fieldA":"Chicken Curry"}]}}"###
    );

    Ok(())
}

async fn search_many_fields(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    insta::assert_snapshot!(
      run_query!(&runner, r#"query { findManyTestModel(where: {
              fieldA: { search: "Chicken" }
              fieldB: { search: "Chicken" }
          }) { fieldA, fieldB }}
    "#),
      @r###"{"data":{"findManyTestModel":[{"fieldA":"Chicken Masala","fieldB":"Chicken, Rice, Masala Sauce"},{"fieldA":"Chicken Curry","fieldB":"Chicken, Curry"},{"fieldA":"Caesar Salad","fieldB":"Salad, Chicken, Caesar Sauce"}]}}"###
    );

    Ok(())
}

async fn search_nullable_field(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    insta::assert_snapshot!(
      run_query!(&runner, r#"query { findManyTestModel(where: {
                fieldA: { search: "Chicken" }
                fieldC: { search: "Chicken" }
            }) { fieldA, fieldC }}
      "#),
      @r###"{"data":{"findManyTestModel":[{"fieldA":"Chicken Masala","fieldC":null},{"fieldA":"Chicken Curry","fieldC":null},{"fieldA":"Caesar Salad","fieldC":"Chicken"}]}}"###
    );

    Ok(())
}

async fn search_with_other_filters(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    insta::assert_snapshot!(
      run_query!(&runner, r#"query { findManyTestModel(where: {
                fieldA: { search: "Chicken", startsWith: "Chicken" },
                fieldB: { search: "Chicken" },
                id: { equals: 1 }
            }) { fieldA, fieldB, fieldC }}
      "#),
      @r###"{"data":{"findManyTestModel":[{"fieldA":"Chicken Masala","fieldB":"Chicken, Rice, Masala Sauce","fieldC":null}]}}"###
    );

    Ok(())
}

async fn search_many_fields_not(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    insta::assert_snapshot!(
      run_query!(&runner, r#"query { findManyTestModel(where: {
                NOT: [{ fieldA: { search: "Chicken" } }, { fieldB: { search: "Carrot" } }]
            }) { id, fieldA, fieldB }}
      "#),
      @r###"{"data":{"findManyTestModel":[{"id":3,"fieldA":"Caesar Salad","fieldB":"Salad, Chicken, Caesar Sauce"},{"id":4,"fieldA":"Beef Sandwich","fieldB":"Bread, Beef"}]}}"###
    );

    Ok(())
}

async fn ensure_filter_tree_shake_works(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    insta::assert_snapshot!(
      run_query!(&runner, r#"query { findManyTestModel(where: {
                AND: [
                    { fieldA: { search: "Chicken", startsWith: "Chicken" } },
                    { OR: [{ fieldB: { search: "Chicken" } }, { id: { equals: 3 } }] }
                ]
            }) { id, fieldA, fieldB, fieldC }}
      "#),
      @r###"{"data":{"findManyTestModel":[{"id":1,"fieldA":"Chicken Masala","fieldB":"Chicken, Rice, Masala Sauce","fieldC":null},{"id":2,"fieldA":"Chicken Curry","fieldB":"Chicken, Curry","fieldC":null}]}}"###
    );

    insta::assert_snapshot!(
      run_query!(&runner, r#"query { findManyTestModel(where: {
                  AND: [
                    { NOT: [{ fieldA: { search: "Chicken" } }, { fieldB: { search: "Beef" } }] },
                    { fieldA: { search: "Salad" } }
                  ]
              }) { id, fieldA, fieldB, fieldC }}
        "#),
      @r###"{"data":{"findManyTestModel":[{"id":3,"fieldA":"Caesar Salad","fieldB":"Salad, Chicken, Caesar Sauce","fieldC":"Chicken"}]}}"###
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
    create_row(
        runner,
        r#"{ id: 4, fieldA: "Beef Sandwich", fieldB: "Bread, Beef", fieldC: "Beef"}"#,
    )
    .await?;

    Ok(())
}

async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
    runner
        .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
        .await?
        .assert_success();
    Ok(())
}

#[test_suite(schema(schema), capabilities(NativeFullTextSearchWithoutIndex))]
mod search_filter_without_index {
    use indoc::indoc;

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
    async fn search_single_field(runner: Runner) -> TestResult<()> {
        super::search_single_field(runner).await
    }

    #[connector_test]
    async fn search_many_fields(runner: Runner) -> TestResult<()> {
        super::search_many_fields(runner).await
    }

    #[connector_test]
    async fn search_nullable_field(runner: Runner) -> TestResult<()> {
        super::search_nullable_field(runner).await
    }

    #[connector_test]
    async fn search_with_other_filters(runner: Runner) -> TestResult<()> {
        super::search_with_other_filters(runner).await
    }

    #[connector_test]
    async fn search_many_fields_not(runner: Runner) -> TestResult<()> {
        super::search_many_fields_not(runner).await
    }

    #[connector_test]
    async fn ensure_filter_tree_shake_works(runner: Runner) -> TestResult<()> {
        super::ensure_filter_tree_shake_works(runner).await
    }
}

#[test_suite(schema(schema), capabilities(NativeFullTextSearchWithIndex))]
mod search_filter_with_index {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              fieldA  String
              fieldB  String
              fieldC  String?
              @@fulltext([fieldA])
              @@fulltext([fieldB])
              @@fulltext([fieldA, fieldB])
              @@fulltext([fieldA, fieldC])
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn search_single_field(runner: Runner) -> TestResult<()> {
        super::search_single_field(runner).await
    }

    #[connector_test]
    async fn search_many_fields(runner: Runner) -> TestResult<()> {
        super::search_many_fields(runner).await
    }

    #[connector_test]
    async fn search_nullable_field(runner: Runner) -> TestResult<()> {
        super::search_nullable_field(runner).await
    }

    #[connector_test]
    async fn search_with_other_filters(runner: Runner) -> TestResult<()> {
        super::search_with_other_filters(runner).await
    }

    #[connector_test]
    async fn search_many_fields_not(runner: Runner) -> TestResult<()> {
        super::search_many_fields_not(runner).await
    }

    #[connector_test]
    async fn ensure_filter_tree_shake_works(runner: Runner) -> TestResult<()> {
        super::ensure_filter_tree_shake_works(runner).await
    }

    #[connector_test]
    async fn throws_error_on_missing_index(runner: Runner) -> TestResult<()> {
        super::create_test_data(&runner).await?;

        assert_error!(
            &runner,
            "query { findManyTestModel(where: {fieldC: { search: \"Chicken\" }}) { id, fieldC }}",
            2030,
            "Cannot find a fulltext index"
        );

        Ok(())
    }
}
