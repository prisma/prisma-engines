use migration_engine_tests::sql::*;
use prisma_value::PrismaValue;
use quaint::Value;
use sql_schema_describer::DefaultValue;

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

    api.infer_apply(&dm2)
        .send()
        .await?
        .assert_no_warning()?
        .assert_unexecutable(&[
            "Made the column `age` on table `Test` required, but there are 1 existing NULL values.".into(),
        ])?;

    api.assert_schema()
        .await?
        .assert_table("Test", |table| table.assert_does_not_have_column("Int"))?;

    let rows = api.select("Test").column("id").column("name").send_debug().await?;
    assert_eq!(rows, &[&[r#"Text(Some("abc"))"#, r#"Text(Some("george"))"#]]);

    Ok(())
}

#[test_each_connector(tags("sqlite"))]
async fn making_an_optional_field_required_with_data_with_a_default_works(api: &TestApi) -> TestResult {
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

    api.infer_apply(&dm2).force(Some(true)).send().await?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table.assert_column("age", |column| {
            column
                .assert_is_required()?
                .assert_default(Some(DefaultValue::VALUE(PrismaValue::Int(84))))
        })
    })?;

    let rows = api
        .select("Test")
        .column("id")
        .column("name")
        .column("age")
        .send()
        .await?;

    assert_eq!(
        rows.into_iter()
            .map(|row| row.into_iter().collect::<Vec<Value>>())
            .collect::<Vec<_>>(),
        &[
            &[Value::text("abc"), Value::text("george"), Value::integer(84)],
            &[Value::text("def"), Value::text("X Æ A-12"), Value::integer(7)],
        ]
    );
    Ok(())
}

// CONFIRMED: this is unexecutable on postgres
// CONFIRMED: all mysql versions except 5.6 will return an error. 5.6 will just insert 0s, which
// seems very wrong, so we should warn against it.
#[test_each_connector(log = "debug", ignore("sqlite"))]
async fn making_an_optional_field_required_with_data_with_a_default_is_unexecutable(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
            age Int?
        }
    "#;

    api.infer_apply(&dm1).send().await?.assert_green()?;

    let initial_schema = api.assert_schema().await?.into_schema();

    api.insert("Test")
        .value("id", "abc")
        .value("name", "george")
        .result_raw()
        .await?;

    api.insert("Test")
        .value("id", "def")
        .value("name", "X Æ A-12")
        .value("age", 7i64)
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
        .force(Some(false))
        .send()
        .await?
        .assert_unexecutable(&[
            "Made the column `age` on table `Test` required, but there are 1 existing NULL values.".into(),
        ])?
        .assert_no_warning()?
        .assert_no_error()?;

    api.assert_schema().await?.assert_equals(&initial_schema)?;

    let rows = api
        .select("Test")
        .column("id")
        .column("name")
        .column("age")
        .send()
        .await?;

    assert_eq!(
        rows.into_iter()
            .map(|row| row.into_iter().collect::<Vec<Value>>())
            .collect::<Vec<_>>(),
        &[
            &[Value::text("abc"), Value::text("george"), Value::Integer(None)],
            &[Value::text("def"), Value::text("X Æ A-12"), Value::integer(7)],
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
