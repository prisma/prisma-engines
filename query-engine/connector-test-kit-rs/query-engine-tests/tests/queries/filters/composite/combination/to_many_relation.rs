//! A lot of those tests are fairly basic - the test matrix is complicated as it is, so these tests strive to cover
//! the API surface expected to be hit most (complexity with little hops) and makes sure that more complex hop combinations
//! do not immediately produce syntax errors / invalid queries.

use super::*;

// [x] To-many-rel -> to-one-com (every, some, none, equals)
// [ ] To-many-rel -> to-one-com -> to-one-com
// [ ] To-many-rel -> to-one-com -> to-many-com
// [ ] To-many-rel -> to-one-com -> scalar list
//
// [ ] To-many-rel -> to-many-com
// [ ] To-many-rel -> to-many-com -> to-one-com
// [ ] To-many-rel -> to-many-com -> to-many-com
// [ ] To-many-rel -> to-many-com -> scalar list
#[test_suite(schema(mixed_composites), only(MongoDb))]
mod to_many_rel {
    // To-many-rel -> to-one-com
    // Every
    #[connector_test]
    async fn to_to_one_com_basic_every(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_many_rel: {
                      every: {
                          to_one_com: {
                              is: {
                                  a_1: {
                                    equals: "test"
                                    mode: insensitive
                                  }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_rel: {
                        every: {
                            to_one_com: {
                                isNot: {
                                    a_1: {
                                      equals: "test"
                                      mode: insensitive
                                    }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":4},{"id":5},{"id":6}]}}"###
        );

        // Explanation:
        // - ID 4 has explicit null values for to_one_com
        // - ID 6 has undefined to_many_rel, which automatically fulfills the condition. (Todo: But should it?)
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_rel: {
                        every: {
                            to_one_com: {
                                is: null
                            }
                        }
                    }
                }) {
                    id
                }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":4},{"id":6}]}}"###
        );

        // All arrays but 4 (which has explicit nulls) automatically fulfill this condition.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_rel: {
                          every: {
                              to_one_com: {
                                  isNot: null
                              }
                          }
                      }
                  }) {
                      id
                  }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    // To-many-rel -> to-one-com
    // Some
    #[connector_test]
    async fn to_to_one_com_basic_some(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_many_rel: {
                      some: {
                          to_one_com: {
                              is: {
                                  a_1: {
                                    equals: "test"
                                  }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_rel: {
                        some: {
                            to_one_com: {
                                isNot: {
                                    a_1: {
                                      equals: "test"
                                    }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"###
        );

        Ok(())
    }

    // To-many-rel -> to-one-com
    // None
    #[connector_test]
    async fn to_to_one_com_basic_none(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_many_rel: {
                      none: {
                          to_one_com: {
                              equals: {
                                  a_1: {
                                    equals: "test"
                                  }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_rel: {
                        none: {
                            to_one_com: {
                                isNot: {
                                    a_1: {
                                      equals: "test"
                                    }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":4},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    // To-many-rel -> to-one-com
    // None
    #[connector_test]
    async fn to_to_one_com_scalar_list(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        // todo: this considers undefined, I'm not sure it should (same with the second test).
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_many_rel: {
                      every: {
                          to_one_com: {
                              is: {
                                  scalar_list: { has: "foo" }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_rel: {
                        every: {
                            to_one_com: {
                                isNot: {
                                    scalar_list: { has: "foo" }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":6}]}"###
        );

        Ok(())
    }
}
