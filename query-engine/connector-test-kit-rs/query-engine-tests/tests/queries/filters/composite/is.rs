use super::*;

#[test_suite(schema(to_one_composites), only(MongoDb))]
mod is {
    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  a: {
                      is: {
                          a_2: { lt: 10 }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    a: {
                        isNot: {
                            a_2: { lt: 10 }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":5}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      a: {
                          is: {}
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        // Todo: This looks bad, but is actually correct in the way we have to build the query.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        a: {
                            isNot: {}
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn multiple_and(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      a: {
                          is: {
                              a_1: { contains: "oo" }
                              a_2: { lt: 10 }
                          }
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Explicit
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        a: {
                            is: {
                                AND: [
                                    { a_1: { contains: "oo" } },
                                    { a_2: { lt: 10 } }
                                ]
                            }
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn multiple_or(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        a: {
                            is: {
                                OR: [
                                    { a_1: { contains: "oo" } },
                                    { a_2: { lt: 10 } }
                                ]
                            }
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":6}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn not_combinations(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        NOT: [
                            {
                                a: {
                                    is: {
                                        a_1: { contains: "oo" }
                                        a_2: { lt: 10 }
                                    }
                                }
                            }
                        ]
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                        findManyTestModel(where: {
                            a: {
                                isNot: {
                                    a_1: { contains: "oo" }
                                    a_2: { lt: 10 }
                                }
                            }
                        }) {
                            id
                        }
                    }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                      findManyTestModel(where: {
                          NOT: [
                              {
                                  a: {
                                      is: {
                                          OR: [
                                              { a_1: { contains: "oo" } },
                                              { a_2: { lt: 10 } }
                                          ]
                                      }
                                  }
                              }
                          ]
                      }) {
                          id
                      }
                  }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":5}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                          findManyTestModel(where: {
                              a: {
                                  isNot: {
                                      OR: [
                                          { a_1: { contains: "oo" } },
                                          { a_2: { lt: 10 } }
                                      ]
                                  }
                              }
                          }) {
                              id
                          }
                      }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":5}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn multiple_hops(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        a: {
                            is: {
                                b: {
                                    is: { b_field: "test" }
                                }
                            }
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                      findManyTestModel(where: {
                          NOT: [
                            {
                                a: {
                                    is: {
                                        b: {
                                            is: { b_field: "test" }
                                        }
                                    }
                                }
                            }
                          ]
                      }) {
                          id
                      }
                  }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                      findManyTestModel(where: {
                          a: {
                              is: {
                                  b: {
                                      is: {
                                          OR: [
                                              { b_field: "test" },
                                              { c: { c_field: "c_field default" } },
                                          ]
                                      }
                                  }
                              }
                          }
                      }) {
                          id
                      }
                  }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyTestModel(
              where: {
                a: {
                  is: {
                    NOT: [
                      {
                        b: {
                          is: {
                            OR: [
                              { b_field: "test" }
                              { c: { c_field: "c_field default" } }
                            ]
                          }
                        }
                      }
                    ]
                  }
                }
              }
            ) {
              id
            }
          }
          "#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn insensitive_must_work(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        a: {
                            is: {
                                a_1: { contains: "Test", mode: insensitive }
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
}
