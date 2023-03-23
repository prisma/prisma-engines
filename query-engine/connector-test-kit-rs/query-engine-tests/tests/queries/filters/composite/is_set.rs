use super::*;

#[test_suite(schema(to_one_composites), only(MongoDb))]
mod is_set_to_one {

    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { b: { isSet: false } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { b: { isSet: true } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn negation(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { NOT: { b: { isSet: false } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { NOT: { b: { isSet: true } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn with_null_check(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { AND: [
                { b: { isSet: true } },
                { b: { isNot: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { AND: [
                { b: { isSet: true } },
                { b: { is: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { AND: [
                { b: { isSet: false } },
                { b: { isNot: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // Field cannot be `undefined` and `null` at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { AND: [
                { b: { isSet: false } },
                { b: { is: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn negation_with_null_check(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        // Field cannot be `undefined` and `null` at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { NOT: [
                { b: { isSet: true } },
                { b: { isNot: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { NOT: [
                { b: { isSet: true } },
                { b: { is: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { NOT: [
                { b: { isSet: false } },
                { b: { isNot: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { NOT: [
                { b: { isSet: false } },
                { b: { is: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn with_equality_check(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { b: { isSet: true } },
                { b: { is: { b_field: "b_1" } } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // `b: null` is excluded because `isNot: { b_field: "b_1"}` checks that `b.b_field` is _not_ `undefined`
        // and `null.b_field` == undefined
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { b: { isSet: true } },
                { b: { isNot: { b_field: "b_1" } } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // Field cannot be `undefined` and something else at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { b: { isSet: false } },
                { b: { is: { b_field: "b_1" } } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // Field cannot be `undefined` and something else at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { b: { isSet: false } },
                { b: { isNot: { b_field: "b_1" } } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn silly_logical_combinations(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              OR: [
                { b: { isSet: false } },
                { b: { isSet: true } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { b: { isSet: false } },
                { b: { isSet: true } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              NOT: [
                { b: { isSet: false } },
                { b: { isSet: true } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_on_required(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { a: { isSet: true } }) { id } }"#,
            2009
        );

        Ok(())
    }
}

#[test_suite(schema(to_one_composites), only(MongoDb))]
mod is_set_scalar {

    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { field: { isSet: false } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { field: { isSet: true } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn negation(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { NOT: { field: { isSet: false } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { NOT: { field: { isSet: true } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn with_null_check(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { AND: [
                { field: { isSet: true } },
                { field: { not: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { AND: [
                { field: { isSet: true } },
                { field: { equals: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6}]}}"###
        );

        // Field cannot be `undefined` and have a value at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { AND: [
                { field: { isSet: false } },
                { field: { not: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // Field cannot be `undefined` and `null` at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { AND: [
                { field: { isSet: false } },
                { field: { equals: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn negation_with_null_check(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        // Field cannot be `undefined` and `null` at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { NOT: [
                { field: { isSet: true } },
                { field: { not: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { NOT: [
                { field: { isSet: true } },
                { field: { equals: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { NOT: [
                { field: { isSet: false } },
                { field: { not: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTestModel(
              where: { NOT: [
                { field: { isSet: false } },
                { field: { equals: null } },
              ]}
            ) { id }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn with_equality_check(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { field: { isSet: true } },
                { field: { equals: "1" } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { field: { isSet: true } },
                { field: { not: "1" } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":5},{"id":6}]}}"###
        );

        // Field cannot be `undefined` and something else at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { field: { isSet: false } },
                { field: { equals: "1" } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // Field cannot be `undefined` and something else at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { field: { isSet: false } },
                { field: { not: "1" } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn silly_logical_combinations(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              OR: [
                { field: { isSet: false } },
                { field: { isSet: true } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { field: { isSet: false } },
                { field: { isSet: true } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              NOT: [
                { field: { isSet: false } },
                { field: { isSet: true } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn fails_on_required(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findManyTestModel(where: { id: { isSet: true } }) { id } }"#,
            2009,
            "Field does not exist in enclosing type"
        );

        Ok(())
    }
}

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod is_set_to_many {

    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { to_many_as: { isSet: false } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":8},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { to_many_as: { isSet: true } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn negation(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { NOT: { to_many_as: { isSet: false } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: { NOT: { to_many_as: { isSet: true } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn with_equality_check(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { to_many_as: { isSet: true } },
                { to_many_as: { some: { a_1: { contains: "oo" } } } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { to_many_as: { isSet: true } },
                { to_many_as: { none: { a_1: { contains: "oo" } } } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5},{"id":6},{"id":7}]}}"###
        );

        // Field cannot be `undefined` and something else at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { to_many_as: { isSet: false } },
                { to_many_as: { equals: [] } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // Field cannot be `undefined` and something else at the same time
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { to_many_as: { isSet: false } },
                { to_many_as: { some: { a_1: { contains: "oo" } } } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn silly_logical_combinations(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              OR: [
                { to_many_as: { isSet: false } },
                { to_many_as: { isSet: true } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              AND: [
                { to_many_as: { isSet: false } },
                { to_many_as: { isSet: true } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(
            where: {
              NOT: [
                { to_many_as: { isSet: false } },
                { to_many_as: { isSet: true } },
              ]
            }
          ) { id } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }
}
