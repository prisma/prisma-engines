use crate::tests::test_harness::sql::*;

#[test_each_connector]
async fn making_an_optional_field_required_with_data_without_a_default_is_unexecutable(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
            age Int?
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

    api.assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_does_not_have_column("Int"))?;

    let rows = api.select("Test").column("id").column("name").send_debug().await?;
    assert_eq!(rows, &[&[r#"Text("abc")"#, r#"Text("george")"#]]);

    Ok(())
}

#[test_each_connector]
async fn making_an_optional_field_required_with_data_with_a_default_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
            age Int?
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
            age Int @default(84)
        }
    "#;

    // TODO: flip this
    api.infer_apply(&dm2).send_assert().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_does_not_have_column("Int"))?;

    let rows = api
        .select("Test")
        .column("id")
        .column("name")
        .column("age")
        .send_debug()
        .await?;
    assert_eq!(rows, &[&[r#"Text("abc")"#, r#"Text("george")"#, "Integer(84)"]]);

    Ok(())
}

#[test_each_connector]
async fn making_an_optional_field_required_on_an_empty_table_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
            age Int?
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

    // TODO: flip this
    api.infer_apply(&dm2).send_assert().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_does_not_have_column("Int"))?;

    let rows = api
        .select("Test")
        .column("id")
        .column("name")
        .column("age")
        .send_debug()
        .await?;

    assert!(rows.is_empty());

    Ok(())
}
