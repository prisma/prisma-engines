use crate::tests::test_harness::sql::*;

#[test_each_connector]
async fn adding_a_required_field_to_an_existing_table_with_data_without_a_default_is_unexecutable(
    api: &TestApi,
) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.infer_apply(&dm1).send_assert().await?.assert_green()?;

    api.insert("Test")
        .value("id", "abc")
        .value("name", "george")
        .result_raw()
        .await?;

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int
        }
    "#;

    // TODO: flip this
    api.infer_apply(&dm2).send_assert().await?.assert_green()?;

    let rows = api.select("Test").column("id").column("name").send_debug().await?;
    assert_eq!(rows, &[&[r#"Text("abc")"#, r#"Text("george")"#]]);

    Ok(())
}

#[test_each_connector]
async fn adding_a_required_field_with_a_default_to_an_existing_table_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.infer_apply(&dm1).send_assert().await?.assert_green()?;

    api.insert("Test")
        .value("id", "abc")
        .value("name", "george")
        .result_raw()
        .await?;

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int @default(45)
        }
    "#;

    api.infer_apply(&dm2).send_assert().await?.assert_green()?;

    let rows = api
        .select("Test")
        .column("id")
        .column("name")
        .column("age")
        .send_debug()
        .await?;
    assert_eq!(rows, &[&[r#"Text("abc")"#, r#"Text("george")"#, r#"Integer(45)"#]]);

    Ok(())
}

#[test_each_connector]
async fn adding_a_required_field_without_default_to_an_existing_table_without_data_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.infer_apply(&dm1).send_assert().await?.assert_green()?;

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int   
        }
    "#;

    api.infer_apply(&dm2).send_assert().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_has_column("age"))?;

    Ok(())
}
