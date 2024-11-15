use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(DecimalType))]
mod decimal_filter {
    use query_engine_tests::run_query;

    pub fn schema() -> String {
        let schema = indoc! {
          "model TestModel {
            #id(id, Int, @id)
            dec     Decimal?
            dec2    Decimal?
          }"
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn basic_where(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { equals: { _ref: "dec", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { equals: { _ref: "dec2", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: { dec: { not: { equals: { _ref: "dec2", _container: "TestModel" } } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn numeric_comparison_filters(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // Gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { gt: { _ref: "dec", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec2: { gt: { _ref: "dec", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not gt => lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { gt: { _ref: "dec", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { gt: { _ref: "dec2", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { gte: { _ref: "dec", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec2: { gte: { _ref: "dec", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not gte => lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { gte: { _ref: "dec", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { gte: { _ref: "dec2", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { lt: { _ref: "dec", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { lt: { _ref: "dec2", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not lt => gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { lt: { _ref: "dec", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec2: { not: { lt: { _ref: "dec", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { lte: { _ref: "dec", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { lte: { _ref: "dec2", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not lte => gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { lte: { _ref: "dec", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec2: { not: { lte: { _ref: "dec", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    pub fn schema_list() -> String {
        let schema = indoc! {
          "model TestModel {
          #id(id, Int, @id)
          dec     Decimal?
          dec_list    Decimal[]
          dec_list2    Decimal[]
        }"
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_list), capabilities(DecimalType, ScalarLists))]
    async fn inclusion_filter(runner: Runner) -> TestResult<()> {
        test_list_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { in: { _ref: "dec_list", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { notIn: { _ref: "dec_list", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { in: { _ref: "dec_list", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_list), capabilities(DecimalType, ScalarLists))]
    async fn scalar_list_filters(runner: Runner) -> TestResult<()> {
        test_list_data(&runner).await?;

        // has
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec_list: { has: { _ref: "dec", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not has
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { dec_list: { has: { _ref: "dec", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // hasSome
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec_list: { hasSome: { _ref: "dec_list", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec_list: { hasSome: { _ref: "dec_list2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // not hasSome
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { dec_list: { hasSome: { _ref: "dec_list", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { dec_list: { hasSome: { _ref: "dec_list2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // hasEvery
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec_list: { hasEvery: { _ref: "dec_list", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec_list: { hasEvery: { _ref: "dec_list2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not hasEvery
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { dec_list: { hasEvery: { _ref: "dec_list", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { dec_list: { hasEvery: { _ref: "dec_list2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    pub async fn test_data(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
              id: 1,
              dec: "1.2",
              dec2: "1.2"
            }) { id }}"#
        );

        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
                id: 2,
                dec: "1.2",
                dec2: "2.4",
            }) { id }}"#
        );

        run_query!(runner, r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"#);

        Ok(())
    }

    pub async fn test_list_data(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
                id: 1,
                dec: "1.5",
                dec_list: ["1.5", "2.4"],
                dec_list2: ["1.5", "2.4"],
            }) { id }}"#
        );
        run_query!(
            runner,
            r#"mutation { createOneTestModel(data: {
                id: 2,
                dec: "1.2",
                dec_list: ["1.5", "2.4"],
                dec_list2: ["2.4", "3.7"],
            }) { id }}"#
        );

        run_query!(runner, r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"#);

        Ok(())
    }
}
