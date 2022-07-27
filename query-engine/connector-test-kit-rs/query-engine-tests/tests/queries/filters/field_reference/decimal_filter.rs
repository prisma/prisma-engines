use query_engine_tests::*;

#[test_suite(capabilities(DecimalType))]
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

    #[connector_test(schema(schema))]
    async fn basic_where(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { equals: { ref: "dec" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { equals: { ref: "dec2" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: { dec: { not: { equals: { ref: "dec2" } } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema))]
    async fn numeric_comparison_filters(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // Gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { gt: { ref: "dec" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec2: { gt: { ref: "dec" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not gt => lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { gt: { ref: "dec" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { gt: { ref: "dec2" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { gte: { ref: "dec" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec2: { gte: { ref: "dec" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not gte => lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { gte: { ref: "dec" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { gte: { ref: "dec2" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { lt: { ref: "dec" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { lt: { ref: "dec2" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not lt => gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { lt: { ref: "dec" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec2: { not: { lt: { ref: "dec" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { lte: { ref: "dec" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { lte: { ref: "dec2" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not lte => gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { lte: { ref: "dec" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec2: { not: { lte: { ref: "dec" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    pub fn schema_list() -> String {
        let schema = indoc! {
          "model TestModel {
          #id(id, Int, @id)
          dec     Decimal?
          dec2    Decimal[]
        }"
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_list), capabilities(ScalarLists))]
    async fn inclusion_filter(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: {
              id: 1,
              dec: "1.2",
              dec2: ["1.2"]
            }) { id }}"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: {
              id: 2,
              dec: "1.2",
              dec2: ["2.4"]
            }) { id }}"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { in: { ref: "dec2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { notIn: { ref: "dec2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { dec: { not: { in: { ref: "dec2" } } } }) { id }}"#),
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
}
