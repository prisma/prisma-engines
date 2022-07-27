use super::setup;
use query_engine_tests::*;

#[test_suite]
mod int_filter {
    use super::setup;
    use query_engine_tests::run_query;

    #[connector_test(schema(setup::common_types))]
    async fn basic_where(runner: Runner) -> TestResult<()> {
        setup::test_data_common_types(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { equals: { ref: "int" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { equals: { ref: "int2" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: { int: { not: { equals: { ref: "int2" } } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_types))]
    async fn numeric_comparison_filters(runner: Runner) -> TestResult<()> {
        setup::test_data_common_types(&runner).await?;

        // Gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { gt: { ref: "int" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int2: { gt: { ref: "int" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not gt => lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { not: { gt: { ref: "int" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { not: { gt: { ref: "int2" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { gte: { ref: "int" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int2: { gte: { ref: "int" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not gte => lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { not: { gte: { ref: "int" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { not: { gte: { ref: "int2" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { lt: { ref: "int" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { lt: { ref: "int2" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not lt => gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { not: { lt: { ref: "int" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int2: { not: { lt: { ref: "int" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { lte: { ref: "int" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { lte: { ref: "int2" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not lte => gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { not: { lte: { ref: "int" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int2: { not: { lte: { ref: "int" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_mixed_types), capabilities(ScalarLists))]
    async fn inclusion_filter(runner: Runner) -> TestResult<()> {
        setup::test_data_common_mixed_types(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { in: { ref: "int2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { notIn: { ref: "int2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { int: { not: { in: { ref: "int2" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }
}
