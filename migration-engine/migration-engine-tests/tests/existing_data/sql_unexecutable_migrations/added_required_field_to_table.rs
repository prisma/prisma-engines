use migration_engine_tests::sql::*;

#[test_each_connector(tags("sql"))]
async fn adding_a_required_field_to_an_existing_table_with_data_without_a_default_is_unexecutable(
    api: &TestApi,
) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.infer_apply(&dm1).send().await?.assert_green()?;

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

    api.infer_apply(&dm2)
        .force(Some(false))
        .send()
        .await?
        .assert_no_warning()?
        .assert_unexecutable(&["Added the required column `age` to the `Test` table without a default value. There are 1 rows in this table, it is not possible to execute this migration.".to_string()])?;

    let rows = api.select("Test").column("id").column("name").send_debug().await?;
    assert_eq!(rows, &[&[r#"Text(Some("abc"))"#, r#"Text(Some("george"))"#]]);

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn adding_a_required_field_with_a_default_to_an_existing_table_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.infer_apply(&dm1).send().await?.assert_green()?;

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

    api.infer_apply(&dm2).send().await?.assert_green()?;

    let rows = api
        .select("Test")
        .column("id")
        .column("name")
        .column("age")
        .send_debug()
        .await?;

    assert_eq!(
        rows,
        &[&[
            r#"Text(Some("abc"))"#,
            r#"Text(Some("george"))"#,
            r#"Integer(Some(45))"#
        ]]
    );

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

    api.infer_apply(&dm1).send().await?.assert_green()?;

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int
        }
    "#;

    api.infer_apply(&dm2).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_has_column("age"))?;

    Ok(())
}
