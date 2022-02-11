use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(to_one_composites), only(MongoDb))]
mod to_one {
    /// Order a model based on a single to-one composite hop.
    #[connector_test]
    async fn model_basic_ordering_single(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Required, ASC
        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { a: { a_1: asc } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        // Required, DESC
        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { a: { a_1: desc } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":5},{"id":4},{"id":3},{"id":2},{"id":1}]}}"###
        );

        // Optional, ASC (nulls appear first)
        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { b: { b_field: asc } }) { id b { b_field } } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":4,"b":null},{"id":5,"b":null},{"id":1,"b":{"b_field":"1_b_field"}},{"id":2,"b":{"b_field":"2_b_field"}},{"id":3,"b":{"b_field":"3_b_field"}}]}}"###
        );

        // Optional, DESC (nulls appear last)
        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { b: { b_field: desc } }) { id b { b_field } } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":3,"b":{"b_field":"3_b_field"}},{"id":2,"b":{"b_field":"2_b_field"}},{"id":1,"b":{"b_field":"1_b_field"}},{"id":4,"b":null},{"id":5,"b":null}]}}"###
        );

        Ok(())
    }

    /// Order a model based on multiple to-one composite hops.
    #[connector_test]
    async fn model_basic_ordering_multiple(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Required, ASC (nulls first)
        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { b: { c: { c_field: asc }} }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":4},{"id":5},{"id":1},{"id":2},{"id":3}]}}"###
        );

        // Required, DESC (nulls last)
        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { b: { c: { c_field: desc }} }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":3},{"id":2},{"id":1},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    /// Order a model based on different orderings at once.
    #[connector_test]
    async fn model_multi_ordering(runner: Runner) -> TestResult<()> {
        create_multi_order_test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(runner, r#"
            { findManyTestModel(
                orderBy: [
                    { a: { a_1: asc } },
                    { a: { a_2: desc } }
                ]) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":2},{"id":1},{"id":3},{"id":4},{"id":5}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, r#"
            { findManyTestModel(
                orderBy: [
                    { b: { b_field: asc } },
                    { a: { a_1: desc } }
                ]) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":5},{"id":4},{"id":1},{"id":3},{"id":2}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        // All data set (except model and last hop to prevent endless circles).
        create_row(runner, r#"{ id: 1, a: { a_1: "1_a_1", a_2: 1, b: { c:{} } }, b: { b_field: "1_b_field", c: { c_field: "1_c_field" } } }"#,).await?;
        create_row(runner, r#"{ id: 2, a: { a_1: "2_a_1", a_2: 2, b: { c:{} } }, b: { b_field: "2_b_field", c: { c_field: "2_c_field" } } }"#,).await?;
        create_row(runner, r#"{ id: 3, a: { a_1: "3_a_1", a_2: 3, b: { c:{} } }, b: { b_field: "3_b_field", c: { c_field: "3_c_field" } } }"#).await?;

        // All optional data is explicitly null.
        create_row(runner, r#"{ id: 4, a: { a_1: "4_a_1", a_2: null, b: { c:{} } }, b: null }"#).await?;

        // All optional data is not set.
        create_row(runner, r#"{ id: 5, a: { a_1: "5_a_1", b: { c:{} } } }"#).await?;

        Ok(())
    }

    /// Test data for ordering by multiple requires some duplicates in the first ordering keys to be useful.
    #[rustfmt::skip]
    async fn create_multi_order_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, a: { a_1: "a1", a_2: 1, b: { c:{} } }, b: { b_field: "b1", c: { c_field: "1_c_field" } } }"#,).await?;
        create_row(runner, r#"{ id: 2, a: { a_1: "a1", a_2: 2, b: { c:{} } }, b: { b_field: "b2", c: { c_field: "2_c_field" } } }"#,).await?;
        create_row(runner, r#"{ id: 3, a: { a_1: "a2", a_2: 2, b: { c:{} } }, b: { b_field: "b2", c: { c_field: "3_c_field" } } }"#).await?;
        create_row(runner, r#"{ id: 4, a: { a_1: "a3", a_2: null, b: { c:{} } }, b: null }"#).await?;
        create_row(runner, r#"{ id: 5, a: { a_1: "a4", b: { c:{} } } }"#).await?;

        Ok(())
    }
}

#[test_suite(schema(to_many_composites), only(MongoDb))]
mod to_many {
    // Order a composite selection by a single orderBy.
    // todo: Doesn't work yet at all, needs more in-depth code changes.
    //#[connector_test]
    // async fn simple_composite_selection_ordering(runner: Runner) -> TestResult<()> {
    //     create_test_data(&runner).await?;

    //     insta::assert_snapshot!(
    //         run_query!(runner, r#"
    //         { findManyTestModel { id to_many_as(orderBy: { a_1: asc }) { a_1 a_2 } } }"#),
    //         @r###"{"data":{"findManyTestModel":[{"id":2},{"id":1},{"id":3},{"id":4},{"id":5}]}}"###
    //     );

    //     Ok(())
    // }

    // Order a model by a to-many.
    #[connector_test]
    async fn model_basic_ordering_many(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { to_many_as: { _count: asc } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4},{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { to_many_as: { _count: desc } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":2},{"id":1},{"id":3},{"id":4}]}}"###
        );

        Ok(())
    }

    /// Order a model based on to-many reached over to-one composite hops.
    /// This test also catches that to-many composites stay queryable together with aggregate order by.
    #[connector_test]
    async fn model_basic_to_many_ordering_multiple_hops(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { to_one_b: { b_to_many_cs: { _count: asc }} }) { id to_one_b { b_field } } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":3,"to_one_b":{"b_field":10}},{"id":4,"to_one_b":null},{"id":2,"to_one_b":{"b_field":10}},{"id":1,"to_one_b":{"b_field":10}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { to_one_b: { b_to_many_cs: { _count: desc }} }) { id to_one_b { b_field } } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":1,"to_one_b":{"b_field":10}},{"id":2,"to_one_b":{"b_field":10}},{"id":3,"to_one_b":{"b_field":10}},{"id":4,"to_one_b":null}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{
                id: 1,
                to_many_as: [
                    { a_1: "1", a_2: 5 },
                    { a_1: "2", a_2: 2 }
                ],
                to_one_b: {
                    b_to_many_cs: [
                        { c_field: 1, c_to_many_as: [{ a_1: "1", a_2: 1 }, { a_1: "2", a_2: 2 }] },
                        { c_field: 2, c_to_many_as: [{ a_1: "3", a_2: 3 }, { a_1: "4", a_2: 4 }] },
                        { c_field: 3, c_to_many_as: [{ a_1: "5", a_2: 5 }, { a_1: "6", a_2: 6 }] }
                    ]
                }
            }"#,
        )
        .await?;

        create_row(
            runner,
            r#"{
                id: 2,
                to_many_as: [
                    { a_1: "10", a_2: 50 },
                    { a_1: "20", a_2: 20 },
                    { a_1: "200", a_2: 200 }
                ],
                to_one_b: {
                    b_to_many_cs: [
                        { c_field: 10, c_to_many_as: [{ a_1: "10", a_2: 10 }, { a_1: "20", a_2: 20 }] },
                        { c_field: 20, c_to_many_as: [{ a_1: "30", a_2: 30 }, { a_1: "40", a_2: 40 }] },
                    ]
                }
            }"#,
        )
        .await?;

        create_row(
            runner,
            r#"{
                id: 3,
                to_many_as: [],
                to_one_b: {
                    b_to_many_cs: []
                }
            }"#,
        )
        .await?;

        create_row(
            runner,
            r#"{
                id: 4,
                to_many_as: [],
            }"#,
        )
        .await?;

        Ok(())
    }
}

#[test_suite(only(MongoDb))]
mod mixed {
    fn over_to_one_relation() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)

              to_one_rel_id Int?
              to_one_rel RelatedModel? @relation(name: "ToOne", fields: [to_one_rel_id], references: [id])
            }

            model RelatedModel {
                #id(id, Int, @id)

                to_one_a   CompositeA?  @map("to_one_composite")
                to_many_bs CompositeB[] @map("to_many_composite")

                test_model TestModel? @relation(name: "ToOne")
            }

            type CompositeA {
                a_1 String @default("a_1 default") @map("a1")
                a_2 Int?
                a_to_many_bs CompositeB[]
            }

            type CompositeB {
                b_field    Int         @default(10)
                b_to_one_c CompositeC? @map("nested_c")
            }

            type CompositeC {
              c_field String @default("c_field default")
            }
            "#
        };

        schema.to_owned()
    }

    // a_to_many_bs: [
    //     { b_field: 50, b_to_one_c: { c_field: "c_field_1" } },
    //     { b_field: 5, b_to_one_c: { c_field: "c_field_2" } }
    // ]
    // to_many_com: [
    //     { b_field: 100, b_to_one_c: { c_field: "c_field_3" } },
    //     { b_field: 10, b_to_one_c: { c_field: "c_field_1" } }
    // ]

    async fn over_to_one_relation_test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, to_one_rel: { create: { id: 1, to_one_a: { a_1: "2", a_2: 1 }, to_many_bs: [ {}, {}, {} ] }}}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 2, to_one_rel: { create: { id: 2, to_one_a: { a_1: "1", a_2: 2 }, to_many_bs: [] }}}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 3, to_one_rel: { create: { id: 3, to_one_a: { a_1: "2", a_2: 5 }, to_many_bs: [ {}, {} ] }}}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 4, to_one_rel: { create: { id: 4, to_one_a: { a_1: "2", a_2: null }, to_many_bs: [ {}, {}, {}, {} ] }}}"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 5, to_one_rel: { create: { id: 5, to_one_a: { a_1: "2" }}}}"#,
        )
        .await?;

        create_row(runner, r#"{ id: 6 }"#).await?;
        create_row(runner, r#"{ id: 7 }"#).await?;

        Ok(())
    }

    /// Order a model based on composites over a relation.
    #[connector_test(schema(over_to_one_relation))]
    async fn composite_over_rel_ordering(runner: Runner) -> TestResult<()> {
        over_to_one_relation_test_data(&runner).await?;

        // Single orderBy ASC.
        // Result is:
        // - Null relations first: (6, 7)
        // - Rows with data next ASC: (2, 1, 3, 4, 5)
        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { to_one_rel: { to_one_a: { a_1: asc } } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":6},{"id":7},{"id":2},{"id":1},{"id":3},{"id":4},{"id":5}]}}"###
        );

        // Single orderBy DESC.
        // Result is:
        // - Rows with data first ASC: (1, 3, 4, 5, 2)
        // - Null relations next: (6, 7)
        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { to_one_rel: { to_one_a: { a_1: desc } } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3},{"id":4},{"id":5},{"id":2},{"id":6},{"id":7}]}}"###
        );

        // Single orderBy over nullable ASC.
        // Result is:
        // - Null values first: (4, 5)
        // - Null relations next: (6, 7)
        // - Rows with data next ASC: (1, 2, 3)
        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { to_one_rel: { to_one_a: { a_2: asc } } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":4},{"id":5},{"id":6},{"id":7},{"id":1},{"id":2},{"id":3}]}}"###
        );

        // Single orderBy over nullable DESC.
        // Result is:
        // - Rows with data first DESC: (3, 2, 1)
        // - Null relations next: (6, 7)
        // - Null values next: (4, 5)
        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { to_one_rel: { to_one_a: { a_2: desc } } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":3},{"id":2},{"id":1},{"id":4},{"id":5},{"id":6},{"id":7}]}}"###
        );

        Ok(())
    }

    /// Order a model based on composite aggregation over a relation.
    #[connector_test(schema(over_to_one_relation))]
    async fn composite_aggr_over_rel_ordering(runner: Runner) -> TestResult<()> {
        over_to_one_relation_test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { to_one_rel: { to_many_bs: { _count: desc } } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":4},{"id":1},{"id":3},{"id":2},{"id":5},{"id":6},{"id":7}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, r#"{ findManyTestModel(orderBy: { to_one_rel: { to_many_bs: { _count: asc } } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":2},{"id":5},{"id":6},{"id":7},{"id":3},{"id":1},{"id":4}]}}"###
        );

        Ok(())
    }

    fn over_to_many_relation() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              to_many_rel RelatedModel[]
            }

            model RelatedModel {
                #id(id, Int, @id)

                to_one_a   CompositeA?  @map("to_one_composite")
                to_many_bs CompositeB[] @map("to_many_composite")

                test_model_id Int
                test_model    TestModel @relation(fields: [test_model_id], references: [id])
            }

            type CompositeA {
                a_1 String @default("a_1 default") @map("a1")
                a_2 Int?
                a_to_many_bs CompositeB[]
            }

            type CompositeB {
                b_field    Int         @default(10)
                b_to_one_c CompositeC? @map("nested_c")
            }

            type CompositeC {
              c_field String @default("c_field default")
            }
            "#
        };

        schema.to_owned()
    }

    async fn over_to_many_relation_test_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{ id: 1, to_many_rel: {
                create: [
                    { id: 1, to_one_a: { a_1: "1", a_2: 1 }},
                    { id: 2, to_one_a: { a_1: "1", a_2: 2 }},
                    { id: 3, to_one_a: { a_1: "2", a_2: 3 }}
                ]
            }
        }"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 2, to_many_rel: {
                create: [
                    { id: 4, to_one_a: { a_1: "2", a_2: 2 }},
                    { id: 5, to_one_a: { a_1: "3", a_2: 2 }},
                    { id: 6, to_one_a: { a_1: "1", a_2: 2 }}
                ]
            }
        }"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 3, to_many_rel: {
                create: []
            }
        }"#,
        )
        .await?;

        create_row(
            runner,
            r#"{ id: 4, to_many_rel: {
                create: [{ id: 7 }, { id: 8, to_one_a: null }]
            }
        }"#,
        )
        .await?;

        Ok(())
    }

    #[connector_test(schema(over_to_many_relation))]
    // Order a related model on a to-one composite.
    async fn order_related_by_to_one_composite(runner: Runner) -> TestResult<()> {
        over_to_many_relation_test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(runner, r#"
              {
                findManyTestModel {
                  id
                  to_many_rel(orderBy: { to_one_a: { a_1: asc } }) {
                    id
                    to_one_a {
                      a_1
                    }
                  }
                }
              }
            "#),
            @r###"{"data":{"findManyTestModel":[{"id":1,"to_many_rel":[{"id":1,"to_one_a":{"a_1":"1"}},{"id":2,"to_one_a":{"a_1":"1"}},{"id":3,"to_one_a":{"a_1":"2"}}]},{"id":2,"to_many_rel":[{"id":6,"to_one_a":{"a_1":"1"}},{"id":4,"to_one_a":{"a_1":"2"}},{"id":5,"to_one_a":{"a_1":"3"}}]},{"id":3,"to_many_rel":[]},{"id":4,"to_many_rel":[{"id":7,"to_one_a":null},{"id":8,"to_one_a":null}]}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, r#"
              {
                findManyTestModel {
                  id
                  to_many_rel(orderBy: { to_one_a: { a_1: desc } }) {
                    id
                    to_one_a {
                      a_1
                    }
                  }
                }
              }
            "#),
            @r###"{"data":{"findManyTestModel":[{"id":1,"to_many_rel":[{"id":3,"to_one_a":{"a_1":"2"}},{"id":1,"to_one_a":{"a_1":"1"}},{"id":2,"to_one_a":{"a_1":"1"}}]},{"id":2,"to_many_rel":[{"id":5,"to_one_a":{"a_1":"3"}},{"id":4,"to_one_a":{"a_1":"2"}},{"id":6,"to_one_a":{"a_1":"1"}}]},{"id":3,"to_many_rel":[]},{"id":4,"to_many_rel":[{"id":7,"to_one_a":null},{"id":8,"to_one_a":null}]}]}}"###
        );

        Ok(())
    }
}

async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
    runner
        .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
        .await?
        .assert_success();

    Ok(())
}
