use migration_engine_tests::sql::*;

#[test_each_connector]
async fn bytes_columns_are_idempotent(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id String @id
            chipData Bytes
        }
    "#;

    api.schema_push(dm)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;

    api.schema_push(dm).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

#[test_each_connector]
async fn float_columns_are_idempotent(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id String @id
            meowFrequency Float
        }
    "#;

    api.schema_push(dm)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;

    api.schema_push(dm).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

#[test_each_connector]
async fn decimal_columns_are_idempotent(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id String @id
            meowFrequency Decimal
        }
    "#;

    api.schema_push(dm)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;

    api.schema_push(dm).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

#[test_each_connector]
async fn float_to_decimal_is_noop(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id String @id
            meowFrequency Float
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("meowFrequency", |col| col.assert_type_is_decimal())
    })?;

    let dm2 = r#"
        model Cat {
            id String @id
            meowFrequency Decimal
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

#[test_each_connector]
async fn decimal_to_float_is_noop(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id String @id
            meowFrequency Decimal
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("meowFrequency", |col| col.assert_type_is_decimal())
    })?;

    let dm2 = r#"
        model Cat {
            id String @id
            meowFrequency Float
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

#[test_each_connector]
async fn bytes_to_string_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id String @id
            meowData Bytes
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("meowData", |col| col.assert_type_is_bytes())
    })?;

    let dm2 = r#"
        model Cat {
            id String @id
            meowData String
        }
    "#;

    api.schema_push(dm2)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("meowData", |col| col.assert_type_is_string())
    })?;

    Ok(())
}

#[test_each_connector]
async fn string_to_bytes_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id String @id
            meowData Bytes
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("meowData", |col| col.assert_type_is_bytes())
    })?;

    let dm2 = r#"
        model Cat {
            id String @id
            meowData String
        }
    "#;

    api.schema_push(dm2)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("meowData", |col| col.assert_type_is_string())
    })?;

    Ok(())
}
