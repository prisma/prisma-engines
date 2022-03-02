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

        // It's not immediately clear what this query is supposed to do.
        // `NOT` with empty filter returns all records.
        // The result is all records that are either empty or don't have that field.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: { to_many_as: { every: { NOT: { a_1: {contains:"ajajajaja"} } } }}) {
                    id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7},{"id":8},{"id":9}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn multiple_conditions(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

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

    #[connector_test(capabilities(InsensitiveFilters))]
    async fn logical_or(runner: Runner) -> TestResult<()> {
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

    // Missing:
    // - or
    //

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

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?;

        Ok(())
    }
}

// Some

// isEmpty

// None

// combination module
// over to-one etc
