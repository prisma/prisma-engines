use super::setup;
use query_engine_tests::*;

#[test_suite]
mod bytes_filter {
    use super::setup;
    use query_engine_tests::run_query;

    #[connector_test(schema(setup::common_types))]
    async fn basic_where(runner: Runner) -> TestResult<()> {
        setup::test_data_common_types(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { bytes: { equals: { _ref: "bytes", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { bytes: { equals: { _ref: "bytes2", _container: "TestModel" } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { AND: { bytes: { not: { equals: { _ref: "bytes2", _container: "TestModel" } } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_mixed_types), capabilities(ScalarLists))]
    async fn inclusion_filter(runner: Runner) -> TestResult<()> {
        setup::test_data_common_mixed_types(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { bytes: { in: { _ref: "bytes2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { bytes: { notIn: { _ref: "bytes2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { bytes: { not: { in: { _ref: "bytes2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(setup::common_list_types), capabilities(ScalarLists))]
    async fn scalar_list_filters(runner: Runner) -> TestResult<()> {
        setup::test_data_list_common(&runner).await?;

        // has
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { bytes_list: { has: { _ref: "bytes", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not has
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { bytes_list: { has: { _ref: "bytes", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // hasSome
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { bytes_list: { hasSome: { _ref: "bytes_list", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { bytes_list: { hasSome: { _ref: "bytes_list2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // not hasSome
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { bytes_list: { hasSome: { _ref: "bytes_list", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { bytes_list: { hasSome: { _ref: "bytes_list2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // hasEvery
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { bytes_list: { hasEvery: { _ref: "bytes_list", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { bytes_list: { hasEvery: { _ref: "bytes_list2", _container: "TestModel" } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not hasEvery
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { bytes_list: { hasEvery: { _ref: "bytes_list", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { bytes_list: { hasEvery: { _ref: "bytes_list2", _container: "TestModel" } } } }) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }
}
