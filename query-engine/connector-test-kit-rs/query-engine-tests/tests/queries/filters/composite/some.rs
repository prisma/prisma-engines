use super::*;

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod some {
    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_many_as: {
                      some: {
                          a_2: { lt: 0 }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4}]}}"###
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
                      some: {}
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      NOT: [
                          { to_many_as: { some: {} }}
                      ]
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_logical_conditions(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        // `AND` with empty filter returns is a truthy condition, so all records fulfill the condition by default.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { some: { AND: {} } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        // `OR` with empty filter returns is a falsey condition, so no records fulfill the condition by default.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(where: { to_many_as: { some: { OR: [] } }}) {
              id
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { some: { OR: [], NOT: [] } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // `NOT` with empty filter returns is a truthy condition, so all record fulfill the condition by default.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { some: { NOT: {} } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
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
                        some: {
                            a_1: { contains: "oo" }
                            a_2: { lt: 0 }
                        }
                    }
                }) {
                  id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        // Explicit AND
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: {
                          some: {
                              AND: [
                                  { a_1: { contains: "oo" } },
                                  { a_2: { lt: 0 } }
                              ]
                          }
                      }
                  }) {
                    id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
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
                        some: {
                            a_1: { contains: "test", mode: insensitive }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn logical_or(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        create_row(
            &runner,
            r#"{ id: 10, to_many_as: [ { a_1: "foo", a_2: 1 }, { a_1: "test", a_2: 10 } ] }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        some: {
                            OR: [
                                { a_1: { contains: "test" } },
                                { a_2: { lt: 0 } }
                            ]
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3},{"id":4},{"id":10}]}}"###
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
                        some: {
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
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn nested_some(runner: Runner) -> TestResult<()> {
        create_to_many_nested_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        some: {
                            a_to_many_bs: {
                                some: {
                                    b_field: { lt: 0 }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
        );

        Ok(())
    }
}
