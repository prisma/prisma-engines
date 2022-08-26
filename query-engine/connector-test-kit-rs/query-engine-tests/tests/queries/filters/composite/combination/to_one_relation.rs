//! A lot of those tests are fairly basic - the test matrix is complicated as it is, so these tests strive to cover
//! the API surface expected to be hit most (complexity with little hops) and makes sure that more complex hop combinations
//! do not immediately produce syntax errors / invalid queries.

use super::*;

// [X] To-one-rel -> to-one-com
// [X] To-one-rel -> to-one-com -> to-one-com
// [X] To-one-rel -> to-one-com -> to-many-com
// [X] To-one-rel -> to-one-com -> scalar list
//
// [X] To-one-rel -> to-many-com
// [X] To-one-rel -> to-many-com -> to-one-com
// [X] To-one-rel -> to-many-com -> to-many-com
// [X] To-one-rel -> to-many-com -> scalar list
#[test_suite(schema(mixed_composites), only(MongoDb))]
mod to_one_rel {
    // To-one-rel -> to-one-com
    #[connector_test]
    async fn to_to_one_com_basic(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_rel: {
                      is: {
                          to_one_com: {
                              is: {
                                  a_1: { contains: "test" }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        isNot: {
                            to_one_com: {
                                is: {
                                    a_1: { contains: "test" }
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

        // Todo: This doesn't return null or undefined values - is this okay?
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_one_rel: {
                          is: {
                              to_one_com: {
                                  isNot: {
                                      a_1: { contains: "test" }
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

        // Todo: This doesn't return undefined values - inconsistent with relations, also problematic with no way of checking for `isSet` or similar.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                            to_one_com: {
                                is: null
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":4}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_one_rel: {
                          isNot: {
                              to_one_com: {
                                  is: null
                              }
                          }
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        to_one_rel: {
                            is: {
                                to_one_com: {
                                    isNot: null
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

    // To-one-rel -> to-one-com
    // Over to-one relation to a to-one composite, multiple conditions
    #[connector_test]
    async fn to_to_one_com_multiple(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        // Implicit AND
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_rel: {
                      is: {
                          to_one_com: {
                              is: {
                                  a_1: { contains: "hello" }
                                  a_2: { lt: 0 }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // OR
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                            to_one_com: {
                                is: {
                                    OR: [
                                        { a_1: { contains: "test" } },
                                        { a_2: { lt: 0 } }
                                    ]

                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // NOT
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_one_rel: {
                          is: {
                              to_one_com: {
                                  is: {
                                      NOT: [
                                          { a_1: { contains: "test" } },
                                          { a_2: { gt: 0 } }
                                      ]
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

    // To-one-rel -> to-one-com
    // Over to-one relation to a to-one composite, logical conditions
    #[connector_test]
    async fn to_to_one_com_logical_cond(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        // Implicit AND
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_rel: {
                      is: {
                          to_one_com: {
                              is: {
                                  a_1: { contains: "hello" }
                                  a_2: { lt: 0 }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // OR
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                            to_one_com: {
                                is: {
                                    OR: [
                                        { a_1: { contains: "test" } },
                                        { a_2: { lt: 0 } }
                                    ]
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // NOT
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_one_rel: {
                          is: {
                              to_one_com: {
                                  is: {
                                      NOT: [
                                          { a_1: { contains: "test" } },
                                          { a_2: { gt: 0 } }
                                      ]
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

    // Scalar lists over to-one relation and to-one composite.
    // To-one-rel -> to-one-com -> scalar list
    #[connector_test]
    async fn to_to_one_scalar_list(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_rel: {
                      is: {
                          to_one_com: {
                              is: {
                                  scalar_list: {
                                      isEmpty: true
                                  }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        // Undefined lists are NOT considered.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                            to_one_com: {
                                is: {
                                    scalar_list: {
                                        isEmpty: false
                                    }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // This DOES consider undefined due to how the query must be build with the not condition.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        isNot: {
                            to_one_com: {
                                is: {
                                    scalar_list: {
                                        isEmpty: true
                                    }
                                }
                            }
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

    // To-one-rel -> to-one-com -> to-one-com
    #[connector_test]
    async fn to_to_one_com_to_to_one_com(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_rel: {
                      is: {
                          to_one_com: {
                              is: {
                                  a_to_other_com: { is: { c_field: { contains: "oo" } } }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                            to_one_com: {
                                is: {
                                    a_to_other_com: { isNot: { c_field: { contains: "oo" } } }
                                }
                            }
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

    // To-one-rel -> to-one-com -> to-many-com
    #[connector_test]
    async fn to_to_one_com_to_to_many_com(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_rel: {
                      is: {
                          to_one_com: {
                              is: {
                                  other_composites: {
                                      some: {
                                          b_field: { contains: "Shardbearer", mode: insensitive }
                                      }
                                  }
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

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                            to_one_com: {
                                isNot: {
                                    other_composites: {
                                        some: {
                                            b_field: { contains: "Shardbearer", mode: insensitive }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    // To-one-rel -> to-many-com
    // Over to-one relation to a to-many composite
    #[connector_test]
    async fn to_to_many_com_basic(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        // Equals
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                            to_many_com: [
                                    {
                                        b_field: "fof",
                                        to_other_com: { c_field: "test" },
                                        to_other_coms: [ { c_field: "nope" } ]
                                    },
                                    {
                                        b_field: "ofo",
                                        to_other_com: { c_field: "Test" },
                                        to_other_coms: [ { c_field: "test" } ]
                                    }
                                ]
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_one_rel: {
                          isNot: {
                              to_many_com: [
                                    {
                                        b_field: "fof",
                                        to_other_com: { c_field: "test" },
                                        to_other_coms: [ { c_field: "nope" } ]
                                    },
                                    {
                                        b_field: "ofo",
                                        to_other_com: { c_field: "Test" },
                                        to_other_coms: [ { c_field: "test" } ]
                                    }
                                ]
                          }
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4}]}}"###
        );

        // Every
        // For empty lists: Since no element is contained, the list automatically passes the condition.
        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_rel: {
                      is: {
                          to_many_com: {
                              every: {
                                  b_field: { contains: "oo" }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        isNot: {
                            to_many_com: {
                                every: {
                                    b_field: { contains: "oo" }
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

        // None
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                            to_many_com: {
                                none: {
                                    b_field: { contains: "oo" }
                                }
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
                      to_one_rel: {
                          isNot: {
                              to_many_com: {
                                  none: {
                                      b_field: { contains: "oo" }
                                  }
                              }
                          }
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // Some
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        to_one_rel: {
                            is: {
                                to_many_com: {
                                    some: {
                                        b_field: { contains: "fof" }
                                    }
                                }
                            }
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                      findManyTestModel(where: {
                          to_one_rel: {
                              isNot: {
                                  to_many_com: {
                                      some: {
                                          b_field: { contains: "fof" }
                                      }
                                  }
                              }
                          }
                      }) {
                          id
                      }
                  }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4}]}}"###
        );

        Ok(())
    }

    // To-one-rel -> to-many-com -> scalar list
    // Scalar lists over to-one relation and to-many composite.
    #[connector_test]
    async fn scalar_lists_to_one_to_many(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_rel: {
                      is: {
                        to_many_com: {
                              every: {
                                  scalar_list: {
                                      has: "hello"
                                  }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        isNot: {
                          to_many_com: {
                                every: {
                                    scalar_list: {
                                        has: "hello"
                                    }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                          to_many_com: {
                                none: {
                                    scalar_list: {
                                        has: "hello"
                                    }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_one_rel: {
                          isNot: {
                            to_many_com: {
                                  none: {
                                      scalar_list: {
                                          has: "hello"
                                      }
                                  }
                              }
                          }
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                          to_many_com: {
                                some: {
                                    scalar_list: {
                                        has: "world"
                                    }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_one_rel: {
                          isNot: {
                            to_many_com: {
                                  some: {
                                      scalar_list: {
                                          has: "world"
                                      }
                                  }
                              }
                          }
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4}]}}"###
        );

        Ok(())
    }

    // To-one-rel -> to-many-com -> to-one-com
    #[connector_test]
    async fn to_to_many_com_to_to_one_com(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_rel: {
                      is: {
                        to_many_com: {
                              every: {
                                  to_other_com: { is: { c_field: { contains: "test", mode: insensitive } } }
                              }
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
                    to_one_rel: {
                        isNot: {
                          to_many_com: {
                                every: {
                                    to_other_com: { is: { c_field: { contains: "test", mode: insensitive } } }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    // To-one-rel -> to-many-com -> to-many-com
    #[connector_test]
    async fn to_to_many_com_to_to_many_com(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_rel: {
                      is: {
                        to_many_com: {
                              every: {
                                to_other_coms: { every: { c_field: { contains: "oo" } } }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        isNot: {
                          to_many_com: {
                                every: {
                                    to_other_com: { is: { c_field: { contains: "test", mode: insensitive } } }
                                }
                            }
                        }
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }
}
