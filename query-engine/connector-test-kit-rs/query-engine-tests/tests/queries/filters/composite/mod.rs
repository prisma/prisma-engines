pub mod combination;
pub mod equals;
pub mod every;
pub mod is;
pub mod is_empty;
pub mod is_set;
pub mod none;
pub mod some;

use query_engine_tests::*;

/// Basic to-many test data.
#[rustfmt::skip]
async fn create_to_many_test_data(runner: &Runner) -> TestResult<()> {
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

/// Test data with one more to-many hop.
async fn create_to_many_nested_test_data(runner: &Runner) -> TestResult<()> {
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

/// Basic to-one test data.
#[rustfmt::skip]
async fn create_to_one_test_data(runner: &Runner) -> TestResult<()> {
    // A few with full data
    create_row(runner, r#"{ id: 1, field: "1", a: { a_1: "foo1", a_2: 1, b: { b_field: "b_nested_1", c: {} }}, b: { b_field: "b_1", c: {} } }"#).await?;
    create_row(runner, r#"{ id: 2, field: "2", a: { a_1: "foo2", a_2: 2, b: { b_field: "b_nested_2", c: {} }}, b: { b_field: "b_2", c: {} } }"#).await?;

    // Optional root `b` (undefined)
    create_row(runner, r#"{ id: 3, a: { a_1: "test", a_2: 10,  b: { b_field: "test", c: {} }} }"#).await?;
    create_row(runner, r#"{ id: 4, a: { a_1: "ofo",  a_2: -100, b: { b_field: "test", c: {} }} }"#).await?;

    // Explicit `null` root `b`
    create_row(runner, r#"{ id: 5, field: null, a: { a_1: "nope", a_2: 99,  b: { b_field: "bar", c: {} }}, b: null }"#).await?;
    create_row(runner, r#"{ id: 6, field: null, a: { a_1: "epon", a_2: -1,  b: { b_field: "rab", c: {} }}, b: null }"#).await?;


    Ok(())
}

/// Composite combination test data.
#[rustfmt::skip]
async fn create_combination_test_data(runner: &Runner) -> TestResult<()> {
    // A few with full data
    create_row(runner, r#"{
        id: 1,
        to_many_as: [ { a_1: "foo1", a_2: 1 },  { a_1: "foo2", a_2: 10 },  { a_1: "oof", a_2: 100 } ]
        to_one_b: { b_field: 1, b_to_many_cs: [ { c_field: 10 }, { c_field: 20 }, { c_field: 30 }] }
    }"#).await?;

    create_row(runner, r#"{
        id: 2,
        to_many_as: [ { a_1: "test1", a_2: 1 }, { a_1: "test2", a_2: 10 }, { a_1: "test3", a_2: 100 } ]
        to_one_b: { b_field: -1, b_to_many_cs: [ { c_field: 10 }, { c_field: -10 } ] }
    }"#).await?;

    create_row(runner, r#"{
        id: 3,
        to_many_as: [ { a_1: "oof", a_2: 100 }, { a_1: "ofo", a_2: 100 },  { a_1: "oof", a_2: -10 } ]
        to_one_b: { b_field: 10, b_to_many_cs: [ { c_field: 0 }, { c_field: 100 } ] }
    }"#).await?;

    create_row(runner, r#"{
        id: 4,
        to_many_as: [ { a_1: "test", a_2: -5 }, { a_1: "Test", a_2: 0 } ]
        to_one_b: { b_field: -100, b_to_many_cs: [ { c_field: 10 } ] }
    }"#).await?;

    // A few with empty as, but some b
    create_row(runner, r#"{ id: 6, to_many_as: [], to_one_b: { b_field: 2, b_to_many_cs: [ { c_field: 100} ] } }"#).await?;
    create_row(runner, r#"{ id: 7, to_many_as: [], to_one_b: { b_field: -2, b_to_many_cs: [ { c_field: -10} ] } }"#).await?;

    // A few with empty as and no b
    create_row(runner, r#"{ id: 8, to_many_as: [], to_one_b: null }"#).await?;
    create_row(runner, r#"{ id: 9, to_many_as: [], to_one_b: null }"#).await?;

    // A few with no list and no b - this will cause undefined fields!
    create_row(runner, r#"{ id: 10 }"#).await?;
    create_row(runner, r#"{ id: 11 }"#).await?;

    Ok(())
}

/// Composite/Relation combination test data.
#[rustfmt::skip]
async fn create_relation_combination_test_data(runner: &Runner) -> TestResult<()> {
    // A few with full data
    create_row(runner, r#"{
        id: 1,
        to_one_rel: {
            create: {
                id: 1
                to_one_com: {
                    a_1: "test",
                    a_2: 10,
                    scalar_list: ["a", "b", "c"],
                    a_to_other_com: { c_field: "foo" },
                    other_composites: [
                        { b_field: "foo", scalar_list: ["1", "2"], to_other_com: { c_field: "test" } },
                        { b_field: "oof", scalar_list: ["1"],      to_other_com: { c_field: "Test" } }
                    ]
                }
                to_many_com: [
                    {
                        b_field: "foo",
                        scalar_list: [],
                        to_other_coms: []
                    },
                    {
                        b_field: "oof",
                        scalar_list: ["123"],
                        to_other_coms: [ { c_field: "foo" } ]
                    }
                ]
            }
        }
        to_many_rel: {
            create: [
                {
                    id: 2
                    to_one_com: {
                        a_1: "test",
                        a_2: 10,
                        scalar_list: [],
                        a_to_other_com: { c_field: "salad" },
                        other_composites: [
                            { b_field: "foo" },
                            { b_field: "oof" }
                        ]
                    }
                    to_many_com: [
                        {
                            b_field: "ayaya",
                            scalar_list: ["test", "tset"],
                            to_other_com: { c_field: "oida" },
                            to_other_coms: []
                        },
                        {
                            b_field: "ofo",
                            scalar_list: ["bar"],
                            to_other_com: { c_field: "nope" },
                            to_other_coms: []
                        }
                    ]
                },
                {
                    id: 3
                    to_one_com: {
                        a_1: "Test",
                        a_2: -10,
                        scalar_list: [],
                        a_to_other_com: { c_field: "wurst salad" },
                        other_composites: [
                            { b_field: "foo" },
                            { b_field: "oof" }
                        ]
                    }
                    to_many_com: [
                        {
                            b_field: "ding",
                            scalar_list: ["hello", "world"],
                            to_other_com: { c_field: "fof" },
                            to_other_coms: [{ c_field: "test" }, { c_field: "oof" }]
                        },
                        {
                            b_field: "dong",
                            scalar_list: ["foo", "bar"],
                            to_other_com: { c_field: "foo" },
                            to_other_coms: [{ c_field: "Test" }, { c_field: "ofo" }]
                        }
                    ]
                }
            ]
        }
    }"#).await?;

    create_row(runner, r#"{
        id: 2,
        to_one_rel: {
            create: {
                id: 4
                to_one_com: {
                    a_1: "hello world",
                    a_2: -5,
                    scalar_list: ["a", "b"],
                    a_to_other_com: { c_field: "oof" },
                    other_composites: [
                        { b_field: "Shardbearer malenia", scalar_list: [], to_other_com: { c_field: "test" } },
                        { b_field: "is", scalar_list: [], to_other_com: { c_field: "foo" } },
                        { b_field: "overtuned", scalar_list: [], to_other_com: { c_field: "oof" } }
                    ]
                }
                to_many_com: [
                    {
                        b_field: "test",
                        scalar_list: ["hello"],
                        to_other_com: { c_field: "test" },
                        to_other_coms: [ { c_field: "oof" } ] },
                    {
                        b_field: "oof",
                        scalar_list: ["hello", "world"],
                        to_other_com: { c_field: "foo" },
                        to_other_coms: [ { c_field: "foof" } ]
                    }
                ]
            }
        }
        to_many_rel: {
            create: [
                {
                    id: 5
                    to_one_com: {
                        a_1: "tset",
                        a_2: 123,
                        scalar_list: ["foo", "oof"],
                        a_to_other_com: { c_field: "test" },
                        other_composites: [ { b_field: "foo" }, { b_field: "test" } ]
                    }
                    to_many_com: [
                        {
                            b_field: "foo",
                            scalar_list: ["foo", "bar"],
                            to_other_com: { c_field: "test" },
                            to_other_coms: [{ c_field: "ofofoo" }, { c_field: "foo?" }]
                        },
                        {
                            b_field: "ofo",
                            scalar_list: ["foo"],
                            to_other_com: { c_field: "Test" },
                            to_other_coms: [{ c_field: "Test" }]
                        }
                    ]
                },
                {
                    id: 6
                    to_one_com: {
                        a_1: "Test",
                        a_2: -10,
                        a_to_other_com: { c_field: "foo" },
                        scalar_list: ["foo", "bar", "baz"],
                        other_composites: [
                            { b_field: "foo" },
                            { b_field: "oof" }
                        ]
                    }
                    to_many_com: [
                        {
                            b_field: "ding",
                            scalar_list: ["test", "foo"],
                            to_other_com: { c_field: "foo" },
                            to_other_coms: [{ c_field: "bar" }, { c_field: "foo!" }]
                        },
                        {
                            b_field: "dong",
                            scalar_list: ["foo", "bar"],
                            to_other_com: { c_field: "test" },
                            to_other_coms: [{ c_field: "test" }, { c_field: "Test" }]
                        }
                    ]
                }
            ]
        }
    }"#).await?;

    create_row(runner, r#"{
        id: 3,
        to_one_rel: {
            create: {
                id: 7
                to_one_com: {
                    a_1: "world",
                    a_2: 0,
                    scalar_list: [],
                    a_to_other_com: { c_field: "ofo", scalar_list: [] },
                    other_composites: [
                        { b_field: "shardbearer mogh", to_other_com: { c_field: "test" }  },
                        { b_field: "is", scalar_list: ["b"], to_other_com: { c_field: "Test" }  },
                        { b_field: "perfect", scalar_list: ["c"], to_other_com: { c_field: "nope" }  }
                    ]
                }
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
        to_many_rel: {
            create: [
                {
                    id: 8
                    to_one_com: {
                        a_1: "test",
                        a_2: 11,
                        scalar_list: [],
                        a_to_other_com: { c_field: "1" },
                        other_composites: [
                            { b_field: "foo" },
                            { b_field: "oof" }
                        ]
                    }
                    to_many_com: [
                        {
                            b_field: "oof",
                            scalar_list: ["test", "bar"],
                            to_other_com: { c_field: "FOO" },
                            to_other_coms: [{ c_field: "test" }]
                        },
                        {
                            b_field: "foof",
                            scalar_list: ["wurst"],
                            to_other_com: { c_field: "OOF" },
                            to_other_coms: [] }
                    ]
                },
                {
                    id: 9
                    to_one_com: {
                        a_1: "Test",
                        a_2: -10,
                        scalar_list: ["foo"],
                        a_to_other_com: { c_field: "2" },
                        other_composites: [
                            { b_field: "foof" },
                            { b_field: "ofoo" }
                        ]
                    }
                    to_many_com: [
                        {
                            b_field: "dood",
                            scalar_list: ["foo"],
                            to_other_com: { c_field: "foo" },
                            to_other_coms: [{ c_field: "woo" }, { c_field: "sah" }]  },
                        {
                            b_field: "doot",
                            scalar_list: ["dood"],
                            to_other_com: { c_field: "oof" },
                            to_other_coms: [{ c_field: "test" }, { c_field: "TEST" }]
                        }
                    ]
                }
            ]
        }
    }"#).await?;

    // One with empty / null composites
    create_row(runner, r#"{
        id: 4,
        to_one_rel: {
            create: {
                id: 10
                to_one_com: null
                to_many_com: []
            }
        }
        to_many_rel: {
            create: [
                {
                    id: 11
                    to_one_com: null
                    to_many_com: []
                },
                {
                    id: 12
                    to_one_com: null
                    to_many_com: []
                }
            ]
        }
    }"#).await?;

    // One with undefined composites
    create_row(runner, r#"{
        id: 5,
        to_one_rel: {
            create: {
                id: 13
            }
        }
        to_many_rel: {
            create: [
                {
                    id: 14
                },
                {
                    id: 15
                }
            ]
        }
    }"#).await?;


    // One with no related records
    create_row(runner, r#"{
        id: 6,
    }"#).await?;

    Ok(())
}

async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
    runner
        .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
        .await?
        .assert_success();

    Ok(())
}
