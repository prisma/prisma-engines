use super::setup;

use query_engine_tests::*;

#[test_suite(schema(setup::mixed_composite_types), capabilities(CompositeTypes))]
mod composite_filter {
    #[connector_test]
    async fn composite_equality(runner: Runner) -> TestResult<()> {
        setup::test_data_mixed_composite(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { comp: { is: { string: { equals: { _ref: "string", _container: "Composite" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { comp: { is: { string: { equals: { _ref: "string2", _container: "Composite" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { comp: { isNot: { string: { equals: { _ref: "string", _container: "Composite" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { comp: { isNot: { string: { equals: { _ref: "string2", _container: "Composite" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn list_equality(runner: Runner) -> TestResult<()> {
        setup::test_data_mixed_composite(&runner).await?;

        // some
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { comp_list: { some: { string: { equals: { _ref: "string", _container: "Composite" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { comp_list: { some: { string: { equals: { _ref: "string2", _container: "Composite" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not some
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { NOT: { comp_list: { some: { string: { equals: { _ref: "string", _container: "Composite" } } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { NOT: { comp_list: { some: { string: { equals: { _ref: "string2", _container: "Composite" } } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // every
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { comp_list: { every: { string: { equals: { _ref: "string", _container: "Composite" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { comp_list: { every: { string: { equals: { _ref: "string2", _container: "Composite" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // not every
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { NOT: { comp_list: { every: { string: { equals: { _ref: "string", _container: "Composite" } } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { NOT: { comp_list: { every: { string: { equals: { _ref: "string2", _container: "Composite" } } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // none
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { comp_list: { none: { string: { equals: { _ref: "string", _container: "Composite" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { comp_list: { none: { string: { equals: { _ref: "string2", _container: "Composite" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // not none
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { NOT: { comp_list: { none: { string: { equals: { _ref: "string", _container: "Composite" } } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { NOT: { comp_list: { none: { string: { equals: { _ref: "string2", _container: "Composite" } } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }
}
