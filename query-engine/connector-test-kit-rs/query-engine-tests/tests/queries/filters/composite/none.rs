use super::*;

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod none {
    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_many_as: {
                      none: {
                          a_2: { lt: 0 }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
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
                      none: {}
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      NOT: [
                          { to_many_as: { none: {} }}
                      ]
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_logical_conditions(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        // `AND` with empty filter returns is a truthy condition, so all records fulfill the condition by default.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { none: { AND: {} } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        // `OR` with empty filter returns is a falsey condition, so no records fulfill the condition by default.
        // Since all false is true for `none`, it returns all records.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(where: { to_many_as: { none: { OR: [] } }}) {
              id
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        // All records do not fulfill this condition, so it returns all records.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { none: { OR: [], NOT: [] } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        // `NOT` with empty filter returns is a truthy condition, so all record fulfill the condition by default.
        // Logically, it shouldn't return any data, but the way we structure our filters it doesn't affect empty or non-existing arrays.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { none: { NOT: {} } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
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
                        none: {
                            a_1: { contains: "oo" }
                            a_2: { lt: 0 }
                        }
                    }
                }) {
                  id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        // Explicit AND
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: {
                          none: {
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
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
                        none: {
                            a_1: { contains: "test", mode: insensitive }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
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
                        none: {
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
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
                        none: {
                            # Read as: "AND: [{ NOT: [ { a_1: { contains: "oo" }}]}, { NOT: [ { a_1: { contains: "test" }}]}]"
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn nested_none(runner: Runner) -> TestResult<()> {
        create_to_many_nested_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        none: {
                            a_to_many_bs: {
                                none: {
                                    b_field: { gt: 0 }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }
}
