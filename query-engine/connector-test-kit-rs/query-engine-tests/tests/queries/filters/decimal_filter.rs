use super::common_test_data;
use query_engine_tests::*;

#[test_suite(schema(schemas::common_nullable_types))]
mod decimal_filter_spec {
    #[connector_test]
    async fn basic_where(runner: &Runner) -> TestResult<()> {
        common_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { equals: "5.5" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { AND: [{ decimal: { not: "1.0" }}, { decimal: { not: null }}] }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { not: null }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn where_shorthands(runner: &Runner) -> TestResult<()> {
        common_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: "5.5" }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: null }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn inclusion_filter(runner: &Runner) -> TestResult<()> {
        common_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { in: ["5.5", "1.0"] }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { AND: [ { decimal: { notIn: ["1.0"] }}, { decimal: { not: null }} ]}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { AND: [{ decimal: { not: { in: ["1.0"] }}}, { decimal: { not: null }}] }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn numeric_comparison_filters(runner: &Runner) -> TestResult<()> {
        common_test_data(runner).await?;

        // Gt
        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { gt: "1.0" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // Not gt => lte
        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { not: { gt: "1.0" }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Gte
        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { gte: "1.0" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not gte => lt
        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { not: { gte: "5.5" }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Lt
        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { lt: "6" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not lt => gte
        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { not: { lt: "5.5" }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // Lte
        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { lte: "5.5" }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not lte => gt
        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel(where: { decimal: { not: { lte: "1" }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }
}
