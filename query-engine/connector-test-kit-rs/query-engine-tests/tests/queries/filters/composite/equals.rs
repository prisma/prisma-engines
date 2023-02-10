use super::*;

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod to_many {
    #[connector_test]
    async fn basic_equality(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: {
                          equals: [ { a_1: "Test", a_2: 0 } ]
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5}]}}"###
        );

        // Implicit equal
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        to_many_as: [ { a_1: "Test", a_2: 0 } ]
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5}]}}"###
        );

        // Implicit equal shorthand (equivalent to the one above)
        insta::assert_snapshot!(
            run_query!(runner, r#"{
                    findManyTestModel(where: {
                        to_many_as: { a_1: "Test", a_2: 0 }
                    }) {
                        id
                    }
                }"#),
            @r###"{"data":{"findManyTestModel":[{"id":5}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, r#"{
                    findManyTestModel(where: {
                        to_many_as: { equals: { a_1: "Test", a_2: 0 } }
                    }) {
                        id
                    }
                }"#),
            @r###"{"data":{"findManyTestModel":[{"id":5}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        NOT: [
                            {
                                to_many_as: {
                                    equals: [ { a_1: "Test", a_2: 0 } ]
                                }
                            }
                        ]
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":6},{"id":7}]}}"###
        );

        // Implicit
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                      findManyTestModel(where: {
                          NOT: [
                              {
                                  to_many_as: [ { a_1: "Test", a_2: 0 } ]
                              }
                          ]
                      }) {
                          id
                      }
                  }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn field_order_matters(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        // Establish baseline
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        to_many_as: {
                            equals: [ { a_1: "Test", a_2: 0 } ]
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":5}]}}"###
        );

        // Actual test
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: {
                          equals: [ { a_2: 0, a_1: "Test" } ]
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn object_order_matters(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        // Establish baseline
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: {
                          equals: [
                              { a_1: "test", a_2: -5 },
                              { a_1: "Test", a_2: 0 }
                          ]
                      }
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":4}]}}"###
        );

        // Actual test
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        to_many_as: {
                            equals: [
                                { a_1: "Test", a_2: 0 },
                                { a_1: "test", a_2: -5 }
                            ]
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_comparison(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                findManyTestModel(where: {
                    to_many_as: {
                        equals: []
                    }
                }) {
                    id
                }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7}]}}"###
        );

        // Implicit
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      to_many_as: []
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                  findManyTestModel(where: {
                      NOT: [
                          {
                            to_many_as: {
                                equals: []
                            }
                          }
                      ]
                  }) {
                      id
                  }
              }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        // Implicit
        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        NOT: [
                            {
                              to_many_as: []
                            }
                        ]
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    // No object coercion
    #[connector_test]
    async fn single_object(runner: Runner) -> TestResult<()> {
        create_to_many_test_data(&runner).await?;

        assert_error!(
            runner,
            r#"{
                findManyTestModel(where: { to_many_as: { equals: { a_1: "Test", a_2: 0 } }}) {
                    id
                }
            }"#,
            2009,
            "`Query.findManyTestModel.where.TestModelWhereInput.to_many_as.CompositeACompositeListFilter.equals`: Value types mismatch. Have: Object({\"a_1\": Scalar(String(\"Test\")), \"a_2\": Scalar(Int(0))}), want: Object(CompositeAObjectEqualityInput)"
        );

        Ok(())
    }

    fn deep_equality_schema() -> String {
        let schema = indoc! {
            r#"
              model CommentRequiredList {
                #id(id, Int, @id)
            
                country String?
                contents CommentContent[]
              }
            
              type CommentContent {
                text    String
                upvotes CommentContentUpvotes[]
              }
            
              type CommentContentUpvotes {
                vote Boolean
                userId String
              }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(deep_equality_schema))]
    async fn deep_equality_shorthand_should_work(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneCommentRequiredList(data: {
                    id: 1,
                    contents: {
                        text: "hello world",
                        upvotes: { vote: true, userId: "10" }
                    }
                }) {
                    id
                }
            }"#
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{
                findManyCommentRequiredList(
                    where: {
                        contents: {
                            equals: {
                                text: "hello world",
                                upvotes: { vote: true, userId: "10" }
                            }
                        }
                    }
                ) {
                    id
                }
            }"#),
            @r###"{"data":{"findManyCommentRequiredList":[{"id":1}]}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(to_one_composites), only(MongoDb))]
mod to_one {
    #[connector_test]
    async fn basic(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        a: {
                            a_1: "foo1"
                            a_2: 1
                            b: { b_field: "b_nested_1", c: { c_field: "c_field default" } }
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
                          NOT: [{ a: {
                              a_1: "foo1"
                              a_2: 1
                              b: { b_field: "b_nested_1", c: { c_field: "c_field default" } }
                          } }]
                      }) {
                          id
                      }
                  }"#),
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn field_order_matters(runner: Runner) -> TestResult<()> {
        create_to_one_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
                    findManyTestModel(where: {
                        a: {
                            a_2: 1
                            a_1: "foo1"
                            b: { b_field: "b_nested_1", c: { c_field: "c_field default" } }
                        }
                    }) {
                        id
                    }
                }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }
}
