use query_engine_tests::*;

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod every {
    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
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
        create_test_data(&runner).await?;

        // `AND` with empty filter returns is a truthy condition, so all record fulfill the condition by default.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { every: { AND: {} } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { every: { OR: [], NOT: [] } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        // `NOT` with empty filter returns is a truthy condition, so all record fulfill the condition by default.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: { to_many_as: { every: { NOT: {} } }}) {
                id
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn locical_and(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(InsensitiveFilters))]
    async fn insensitive(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

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
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn logical_or(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":6},{"id":7},{"id":8},{"id":9},{"id":10}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn logical_not(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

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
          @r###"{"data":{"findManyTestModel":[{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        // A few with full data
        create_row(runner, r#"{ id: 1, to_many_as: [ { a_1: "foo1", a_2: 1 },  { a_1: "foo2", a_2: 10 },  { a_1: "oof", a_2: 100 }   ] }"#).await?;
        create_row(runner, r#"{ id: 2, to_many_as: [ { a_1: "test1", a_2: 1 }, { a_1: "test2", a_2: 10 }, { a_1: "test3", a_2: 100 } ] }"#).await?;
        create_row(runner, r#"{ id: 3, to_many_as: [ { a_1: "oof", a_2: 100 }, { a_1: "ofo", a_2: 100 },  { a_1: "oof", a_2: -10 }   ] }"#).await?;
        create_row(runner, r#"{ id: 4, to_many_as: [ { a_1: "test", a_2: -5 }, { a_1: "Test", a_2: 0 }                               ] }"#).await?;
        create_row(runner, r#"{ id: 5, to_many_as: [ { a_1: "Test", a_2: 0 }                                                         ] }"#).await?;

        // A few with empty list
        create_row(runner, r#"{ id: 6, to_many_as: [] }"#).await?;
        create_row(runner, r#"{ id: 7, to_many_as: [] }"#).await?;

        // A few with no list - this will cause undefined fields!
        create_row(runner, r#"{ id: 8 }"#).await?;
        create_row(runner, r#"{ id: 9 }"#).await?;

        Ok(())
    }

    #[connector_test]
    async fn nested_every(runner: Runner) -> TestResult<()> {
        create_nested_test_data(&runner).await?;

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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    /// Test data with one more to-many hop.
    async fn create_nested_test_data(runner: &Runner) -> TestResult<()> {
        // A few with full data
        create_row(
            runner,
            r#"
            { id: 1, to_many_as: [
                { a_1: "foo1", a_2: 1, a_to_many_bs:  [ { b_field: 123 }, { b_field: 5 }  ] },
                { a_1: "foo2", a_2: 10, a_to_many_bs: [ { b_field: 321 }, { b_field: 5 }  ] },
                { a_1: "oof", a_2: 100, a_to_many_bs: [ { b_field: 111 }, { b_field: 50 } ] }
            ] }"#,
        )
        .await?;

        create_row(
            runner,
            r#"
            { id: 2, to_many_as: [
                { a_1: "test1", a_2: 1,   a_to_many_bs: [ { b_field: 1 }, { b_field: 2 }  ] },
                { a_1: "test2", a_2: 10,  a_to_many_bs: [ { b_field: 5 }, { b_field: 5 }  ] },
                { a_1: "test3", a_2: 100, a_to_many_bs: [ { b_field: 0 }, { b_field: -5 } ] }
            ] }"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 3, to_many_as: [
                { a_1: "oof", a_2: 100, a_to_many_bs: [ { b_field: 0 }, { b_field: 0 }  ] },
                { a_1: "ofo", a_2: 100, a_to_many_bs: [ { b_field: -2 }, { b_field: 2 } ] },
                { a_1: "oof", a_2: -10, a_to_many_bs: [ { b_field: 1 }, { b_field: 1 }  ] }
            ] }"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 4, to_many_as: [
                { a_1: "test", a_2: -5, a_to_many_bs: [ { b_field: 10 }, { b_field: 20 } ] },
                { a_1: "Test", a_2: 0, a_to_many_bs:  [ { b_field: 11 }, { b_field: 22 } ] }
            ] }"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 5, to_many_as: [{ a_1: "Test", a_2: 0, a_to_many_bs: [ { b_field: 5 }, { b_field: 55 } ] }] }"#,
        )
        .await?;

        // A few with empty list
        create_row(runner, r#"{ id: 6, to_many_as: [] }"#).await?;
        create_row(runner, r#"{ id: 7, to_many_as: [] }"#).await?;

        // A few with no list - this will cause undefined fields!
        create_row(runner, r#"{ id: 8 }"#).await?;
        create_row(runner, r#"{ id: 9 }"#).await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();

        Ok(())
    }
}

// Some

// isEmpty

// None

// combination module
// over to-one etc
