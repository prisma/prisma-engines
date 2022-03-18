use super::*;

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod composite_combination {
    // Over to-one to to-many (both composites)
    #[connector_test]
    async fn com_to_one_2_to_many(runner: Runner) -> TestResult<()> {
        create_combination_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
              findManyTestModel(where: {
                  to_one_b: {
                      is: {
                          b_to_many_cs: {
                              every: {
                                  c_field: { gt: 0 }
                              }
                          }
                      }
                  }
              }) {
                  id
              }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4},{"id":6},{"id":8},{"id":9},{"id":10},{"id":11}]}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(mixed_composites), only(MongoDb))]
mod relation_combination {
    // Over to-one relation to a to-one composite
    #[connector_test]
    async fn over_to_one_rel_to_one(runner: Runner) -> TestResult<()> {
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
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":5},{"id":6}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    // Over to-one relation to a to-one composite, multiple conditions
    #[connector_test]
    async fn over_to_one_rel_to_one_multiple(runner: Runner) -> TestResult<()> {
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

    // Over to-one relation to a to-one composite, multiple conditions
    #[connector_test]
    async fn over_to_one_rel_to_one_logical_cond(runner: Runner) -> TestResult<()> {
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

    // Over to-one relation to a to-many composite
    #[connector_test]
    async fn over_to_one_rel_to_many(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        // Equals
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_one_rel: {
                        is: {
                            to_many_com: [ { b_field: "fof" }, { b_field: "ofo" } ]
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
                              to_many_com: [ { b_field: "fof" }, { b_field: "ofo" } ]
                          }
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5},{"id":6}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4},{"id":5}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3},{"id":6}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4},{"id":5}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":6}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    // Scalar lists over to-one relation and to-one composite.
    #[connector_test]
    async fn scalar_lists_to_one_to_one(runner: Runner) -> TestResult<()> {
        create_relation_combination_test_data(&runner).await?;

        // Todo (to-clarify): This considers null and undefined scalar lists as empty.
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
          @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4},{"id":5}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":6}]}}"###
        );

        Ok(())
    }

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
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5}]}}"###
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
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }
}
