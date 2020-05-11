use migration_engine_tests::sql::*;

#[test_each_connector(tags("sql"))]
async fn making_an_optional_field_required_with_data_without_a_default_is_unexecutable(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
            age Int?
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

    api.infer_apply(&dm2).send().await?.assert_unexecutable(&[
        "Made the column `age` on table `Test` required, but there are existing NULL values.".into(),
    ])?;

    api.assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_does_not_have_column("Int"))?;

    let rows = api.select("Test").column("id").column("name").send_debug().await?;
    assert_eq!(rows, &[&[r#"Text("abc")"#, r#"Text("george")"#]]);

    Ok(())
}

#[test_each_connector(log = "debug,sql_schema_describer=info")]
async fn making_an_optional_field_required_with_data_with_a_default_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
            age Int?
        }
    "#;

    api.infer_apply(&dm1)
        .migration_id(Some("apply-dm1"))
        .send()
        .await?
        .assert_green()?;

    api.insert("Test")
        .value("id", "abc")
        .value("name", "george")
        .result_raw()
        .await?;

    api.insert("Test")
        .value("id", "def")
        .value("name", "X Æ A-12")
        .value("age", 7)
        .result_raw()
        .await?;

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int @default(84)
        }
    "#;

    api.infer_apply(&dm2)
        .migration_id(Some("apply-dm2"))
        .send()
        .await?
        .assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table.assert_column("age", |col| col.assert_is_required())
    })?;

    let rows = api
        .select("Test")
        .column("id")
        .column("name")
        .column("age")
        .send_debug()
        .await?;
    assert_eq!(
        rows,
        &[
            &[r#"Text("abc")"#, r#"Text("george")"#, "Integer(84)"],
            &[r#"Text("def")"#, r#"Text("X Æ A-12")"#, "Integer(7)"],
        ]
    );

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
