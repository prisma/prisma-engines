use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(DecimalType))]
mod decimal_filter_spec {
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
              model TestModel {
                #id(id, Int, @id)
                decimal Decimal?
              }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn basic_where(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { equals: "5.5" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: [{ decimal: { not: "1.0" }}, { decimal: { not: null }}] }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { not: null }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn where_shorthands(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: "5.5" }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        match_connector_result!(
          &runner,
          r#"query { findManyTestModel(where: { decimal: null }) { id }}"#,
          // MongoDB excludes undefined fields
          MongoDb(_) => vec![r#"{"data":{"findManyTestModel":[]}}"#],
          _ => vec![r#"{"data":{"findManyTestModel":[{"id":3}]}}"#]
        );

        Ok(())
    }

    #[connector_test]
    async fn inclusion_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { in: ["5.5", "1.0"] }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: [ { decimal: { notIn: ["1.0"] }}, { decimal: { not: null }} ]}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: [{ decimal: { not: { in: ["1.0"] }}}, { decimal: { not: null }}] }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn numeric_comparison_filters(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // Gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { gt: "1.0" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // Not gt => lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { not: { gt: "1.0" }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { gte: "1.0" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not gte => lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { not: { gte: "5.5" }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { lt: "6" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not lt => gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { not: { lt: "5.5" }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // Lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { lte: "5.5" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not lte => gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { not: { lte: "1" }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc! { r#"
                mutation { createOneTestModel(data: {
                    id: 1,
                    bInt: 5,
                    decimal: "5.5",
                    bytes: "dGVzdA==",
                }) { id }}"# })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                mutation { createOneTestModel(data: {
                    id: 2,
                    bInt: 1,
                    decimal: "1",
                    bytes: "dA==",
                }) { id }}"# })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"# })
            .await?
            .assert_success();

        Ok(())
    }
}
