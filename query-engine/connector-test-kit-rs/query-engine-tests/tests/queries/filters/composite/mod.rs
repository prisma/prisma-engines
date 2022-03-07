pub mod every;
pub mod is_empty;
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

async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
    runner
        .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
        .await?
        .assert_success();

    Ok(())
}
