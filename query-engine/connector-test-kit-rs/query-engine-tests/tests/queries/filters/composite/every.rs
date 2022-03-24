use super::*;

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod every {
    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_many_as: {
                      every: {
                          a_2: { gt: 0 }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_many_as: {
                      every: {}
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    NOT: [
                        { to_many_as: { every: {} }}
                    ]
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_logical_conditions(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        // `AND` with empty filter returns is a truthy condition, so all record fulfill the condition by default.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { every: { AND: {} } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7}]}}"###
        );

        // `OR` with empty filter returns is a falsey condition, so no records fulfill the condition by default.
        // **However**: Empty or non-existing arrays are automatically true due to how we build the conditions,
        // and it's unclear if this is really an incorrect result or not.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(where: { to_many_as: { every: { OR: [] } }}) {
              id
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { every: { OR: [], NOT: [] } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7}]}}"###
        );

        // `NOT` with empty filter returns is a truthy condition, so all record fulfill the condition by default.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { every: { NOT: {} } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn locical_and(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        // Implicit AND
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        every: {
                            a_1: { contains: "oo" }
                            a_2: { gt: 0 }
                        }
                    }
                }) {
                  id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":6},{"id":7}]}}"###
        );

        // Explicit AND
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: {
                          every: {
                              AND: [
                                  { a_1: { contains: "oo" } },
                                  { a_2: { gt: 0 } }
                              ]
                          }
                      }
                  }) {
                    id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(InsensitiveFilters))]
    async fn insensitive(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        every: {
                            a_1: { contains: "test", mode: insensitive }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5},{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn logical_or(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        create_row(
            &runner,
            r#"{ id: 10, to_many_as: [ { a_1: "foo", a_2: 1 },  { a_1: "test", a_2: 10 } ] }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        every: {
                            OR: [
                                { a_1: { contains: "oo" } },
                                { a_1: { contains: "test" } }
                            ]
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":6},{"id":7},{"id":10}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn logical_not(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        every: {
                            NOT: [
                                { a_1: { contains: "oo" } },
                                { a_1: { contains: "test" } }
                            ]
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn nested_every(runner: Runner) -> TestResult<()> {
        create_to_many_nested_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        every: {
                            a_to_many_bs: {
                                every: {
                                    b_field: { gte: 0 }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4},{"id":5},{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }
}
