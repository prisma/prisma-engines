use super::setup;
use query_engine_tests::*;

#[test_suite(schema(setup::common_types))]
mod string_filter {
    use super::setup;
    use query_engine_tests::run_query;

    #[connector_test]
    async fn basic_where_sensitive(runner: Runner) -> TestResult<()> {
        setup::test_data_common_types(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { equals: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { equals: { _ref: "string2", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: { string: { not: { equals: { _ref: "string2", _container: "TestModel" } } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(InsensitiveFilters))]
    async fn basic_where_insensitive(runner: Runner) -> TestResult<()> {
        test_data_insensitive(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, equals: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, equals: { _ref: "string2", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: { string: { mode: insensitive, not: { equals: { _ref: "string2", _container: "TestModel" } } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn numeric_comparison_filters_sensitive(runner: Runner) -> TestResult<()> {
        setup::test_data_common_types(&runner).await?;

        // Gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { gt: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { gt: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not gt => lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { gt: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { gt: { _ref: "string2", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { gte: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { gte: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not gte => lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { gte: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { gte: { _ref: "string2", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { lt: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { lt: { _ref: "string2", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not lt => gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { lt: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { not: { lt: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { lte: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { lte: { _ref: "string2", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not lte => gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { lte: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { not: { lte: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    // FIXME: MongoDB numeric insensitive filters are broken
    #[connector_test(exclude(MongoDB), capabilities(InsensitiveFilters))]
    async fn numeric_comparison_filters_insensitive(runner: Runner) -> TestResult<()> {
        test_data_insensitive(&runner).await?;

        // Gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, gt: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { mode: insensitive, gt: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not gt => lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { gt: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { gt: { _ref: "string2", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, gte: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { mode: insensitive, gte: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not gte => lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { gte: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { gte: { _ref: "string2", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, lt: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, lt: { _ref: "string2", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Not lt => gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { lt: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { mode: insensitive, not: { lt: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, lte: { _ref: "string", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, lte: { _ref: "string2", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Not lte => gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { lte: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string2: { mode: insensitive, not: { lte: { _ref: "string", _container: "TestModel" } }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn string_comparison_filters_sensitive(runner: Runner) -> TestResult<()> {
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
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { contains: { _ref: "string", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { contains: { _ref: "string2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4},{"id":5}]}}"###
        );

        // not contains
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { contains: { _ref: "string", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { contains: { _ref: "string2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // startsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { startsWith: { _ref: "string", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { startsWith: { _ref: "string2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4}]}}"###
        );

        // not startsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { startsWith: { _ref: "string", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { startsWith: { _ref: "string2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":5}]}}"###
        );

        // endsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { endsWith: { _ref: "string", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { endsWith: { _ref: "string2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":5}]}}"###
        );

        // not endsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { endsWith: { _ref: "string", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { endsWith: { _ref: "string2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(InsensitiveFilters))]
    async fn string_comparison_filters_insensitive(runner: Runner) -> TestResult<()> {
        test_data_insensitive(&runner).await?;
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: {
              id: 4,
              string: "aBc",
              string2: "AB",
            }) { id }}"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: {
            id: 5,
            string: "aBC",
            string2: "bC",
          }) { id }}"#
        );

        // contains
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, contains: { _ref: "string", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, contains: { _ref: "string2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4},{"id":5}]}}"###
        );

        // not contains
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { contains: { _ref: "string", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { contains: { _ref: "string2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // startsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, startsWith: { _ref: "string", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, startsWith: { _ref: "string2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4}]}}"###
        );

        // not startsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { startsWith: { _ref: "string", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { startsWith: { _ref: "string2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":5}]}}"###
        );

        // endsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, endsWith: { _ref: "string", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, endsWith: { _ref: "string2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":5}]}}"###
        );

        // not endsWith
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { endsWith: { _ref: "string", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { endsWith: { _ref: "string2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_mixed_types), capabilities(ScalarLists))]
    async fn inclusion_filter_sensitive(runner: Runner) -> TestResult<()> {
        setup::test_data_common_mixed_types(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { in: { _ref: "string2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { notIn: { _ref: "string2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { not: { in: { _ref: "string2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_mixed_types), capabilities(ScalarLists, InsensitiveFilters))]
    async fn inclusion_filter_insensitive(runner: Runner) -> TestResult<()> {
        test_data_mixed_types_insensitive(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, in: { _ref: "string2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, notIn: { _ref: "string2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { mode: insensitive, not: { in: { _ref: "string2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_list_types), capabilities(ScalarLists))]
    async fn scalar_list_filters_sensitive(runner: Runner) -> TestResult<()> {
        setup::test_data_list_common(&runner).await?;

        // has
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string_list: { has: { _ref: "string", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not has
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { string_list: { has: { _ref: "string", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // hasSome
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string_list: { hasSome: { _ref: "string_list", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string_list: { hasSome: { _ref: "string_list2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // not hasSome
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { string_list: { hasSome: { _ref: "string_list", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { string_list: { hasSome: { _ref: "string_list2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // hasEvery
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string_list: { hasEvery: { _ref: "string_list", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string_list: { hasEvery: { _ref: "string_list2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not hasEvery
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { string_list: { hasEvery: { _ref: "string_list", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { string_list: { hasEvery: { _ref: "string_list2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    pub async fn test_data_insensitive(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc! { r#"
              mutation { createOneTestModel(data: {
                  id: 1,
                  string: "abc",
                  string2: "aBC",
              }) { id }}"# })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation { createOneTestModel(data: {
                  id: 2,
                  string: "aBC",
                  string2: "Bcd",
              }) { id }}"# })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"# })
            .await?
            .assert_success();

        Ok(())
    }

    pub async fn test_data_mixed_types_insensitive(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc! { r#"
                mutation { createOneTestModel(data: {
                    id: 1,
                    string: "a",
                    string2: ["A"],
                }) { id }}"# })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation { createOneTestModel(data: {
                  id: 2,
                  string: "a",
                  string2: ["B"],
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
