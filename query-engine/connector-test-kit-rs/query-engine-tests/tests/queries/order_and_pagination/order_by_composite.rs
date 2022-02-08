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
        create_row(runner, r#"{ id: 1, a: { a_1: "1_a_1", a_2: 1 }, b: { b_field: "1_b_field", c: { c_field: "1_c_field" } } }"#,).await?;
        create_row(runner, r#"{ id: 2, a: { a_1: "2_a_1", a_2: 2 }, b: { b_field: "2_b_field", c: { c_field: "2_c_field" } } }"#,).await?;
        create_row(runner, r#"{ id: 3, a: { a_1: "3_a_1", a_2: 3 }, b: { b_field: "3_b_field", c: { c_field: "3_c_field" } } }"#).await?;

        // All optional data is explicitly null.
        create_row(runner, r#"{ id: 4, a: { a_1: "4_a_1", a_2: null }, b: null }"#).await?;

        // All optional data is not set.
        create_row(runner, r#"{ id: 5, a: { a_1: "5_a_1" } }"#).await?;

        Ok(())
    }

    /// Test data for ordering by multiple requires some duplicates in the first ordering keys to be useful.
    #[rustfmt::skip]
    async fn create_multi_order_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, a: { a_1: "a1", a_2: 1 }, b: { b_field: "b1", c: { c_field: "1_c_field" } } }"#,).await?;
        create_row(runner, r#"{ id: 2, a: { a_1: "a1", a_2: 2 }, b: { b_field: "b2", c: { c_field: "2_c_field" } } }"#,).await?;
        create_row(runner, r#"{ id: 3, a: { a_1: "a2", a_2: 2 }, b: { b_field: "b2", c: { c_field: "3_c_field" } } }"#).await?;
        create_row(runner, r#"{ id: 4, a: { a_1: "a3", a_2: null }, b: null }"#).await?;
        create_row(runner, r#"{ id: 5, a: { a_1: "a4" } }"#).await?;

        Ok(())
    }
}

#[test_suite(schema(mixed_composites), only(MongoDb))]
mod mixed {

    /// Order a model based on composites over a relation.
    #[connector_test]
    async fn composite_over_rel_ordering(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(runner, r#"
            { findManyTestModel(orderBy: { to_one_rel: { to_one_com: { a_1: asc } } }) { id } }"#),
            @r###"{"data":{"findManyTestModel":[{"id":2},{"id":1}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        // All data set
        create_row(
            runner,
            r#"
            {
                id: 1,
                to_one_com: {
                    a_1: "a1",
                    a_2: 1,
                    other_composites: [
                        { b_field: "b1", to_other_com: { c_field: "1_c_field" } }
                    ]
                },
                to_many_com: [
                    { b_field: "b3", to_other_com: { c_field: "3_c_field" } },
                    { b_field: "b4", to_other_com: { c_field: "4_c_field" } }
                ]
                to_one_rel: {
                    create: {
                        id: 1,
                        to_one_com: {
                            a_1: "2",
                            a_2: 2,
                            other_composites: [
                                { b_field: "b4", to_other_com: { c_field: "5_c_field" } },
                                { b_field: "b5", to_other_com: { c_field: "6_c_field" } }
                            ]
                        },
                        to_many_com: [
                            { b_field: "b6", to_other_com: { c_field: "7_c_field" } },
                            { b_field: "b7", to_other_com: { c_field: "8_c_field" } }
                        ]
                    }
                }
            }
            "#,
        )
        .await?;

        create_row(
            runner,
            r#"
            {
                id: 2,
                to_one_com: {
                    a_1: "a1",
                    a_2: 1,
                    other_composites: [
                        { b_field: "b1", to_other_com: { c_field: "1_c_field" } },
                        { b_field: "b2", to_other_com: { c_field: "2_c_field" } }
                    ]
                },
                to_many_com: [
                    { b_field: "b3", to_other_com: { c_field: "3_c_field" } },
                    { b_field: "b4", to_other_com: { c_field: "4_c_field" } }
                ]
                to_one_rel: {
                    create: {
                        id: 2,
                        to_one_com: {
                            a_1: "1",
                            a_2: 2,
                            other_composites: [
                                { b_field: "b4", to_other_com: { c_field: "5_c_field" } },
                                { b_field: "b5", to_other_com: { c_field: "6_c_field" } }
                            ]
                        },
                        to_many_com: [
                            { b_field: "b6", to_other_com: { c_field: "7_c_field" } },
                            { b_field: "b7", to_other_com: { c_field: "8_c_field" } }
                        ]
                    }
                }
            }
            "#,
        )
        .await?;

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
