use super::setup;
use query_engine_tests::*;

#[test_suite]
mod string_filter {
    use super::setup;
    use query_engine_tests::run_query;

    #[connector_test(schema(setup::common_types))]
    async fn basic_where(runner: Runner) -> TestResult<()> {
        setup::test_data_common_types(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { equals: { _ref: "string" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { equals: { _ref: "string2" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: { string: { not: { equals: { _ref: "string2" } } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_types))]
    async fn numeric_comparison_filters(runner: Runner) -> TestResult<()> {
        setup::test_data_common_types(&runner).await?;

        // Gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { gt: { _ref: "string" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { gt: { _ref: "string" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not gt => lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { gt: { _ref: "string" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { gt: { _ref: "string2" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { gte: { _ref: "string" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { gte: { _ref: "string" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not gte => lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { gte: { _ref: "string" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { gte: { _ref: "string2" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { lt: { _ref: "string" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { lt: { _ref: "string2" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not lt => gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { lt: { _ref: "string" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { not: { lt: { _ref: "string" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { lte: { _ref: "string" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { lte: { _ref: "string2" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not lte => gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { lte: { _ref: "string" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { not: { lte: { _ref: "string" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_types))]
    async fn string_comparison_filters(runner: Runner) -> TestResult<()> {
        setup::test_data_common_types(&runner).await?;
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: {
              id: 4,
              string: "abc",
              string2: "ab",
            }) { id }}"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: {
            id: 5,
            string: "abc",
            string2: "bc",
          }) { id }}"#
        );

        // contains
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { contains: { _ref: "string" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { contains: { _ref: "string2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4},{"id":5}]}}"###
        );

        // not contains
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { contains: { _ref: "string" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { contains: { _ref: "string2" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // startsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { startsWith: { _ref: "string" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { startsWith: { _ref: "string2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4}]}}"###
        );

        // not startsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { startsWith: { _ref: "string" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { startsWith: { _ref: "string2" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":5}]}}"###
        );

        // endsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { endsWith: { _ref: "string" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { endsWith: { _ref: "string2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":5}]}}"###
        );

        // not endsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { endsWith: { _ref: "string" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { endsWith: { _ref: "string2" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_mixed_types), capabilities(ScalarLists))]
    async fn inclusion_filter(runner: Runner) -> TestResult<()> {
        setup::test_data_common_mixed_types(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { in: { _ref: "string2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { notIn: { _ref: "string2" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { in: { _ref: "string2" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }
}
