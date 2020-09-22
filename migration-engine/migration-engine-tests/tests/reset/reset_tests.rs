use crate::*;

#[test_each_connector]
async fn reset_works(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    api.schema_push(dm).send().await?;

    api.assert_schema().await?.assert_tables_count(1)?;

    api.insert("Cat")
        .value("id", 1)
        .value("name", "Garfield")
        .result_raw()
        .await?;

    api.reset().send().await?;

    api.assert_schema().await?.assert_tables_count(0)?;

    api.schema_push(dm).send().await?;

    api.assert_schema().await?.assert_tables_count(1)?;

    Ok(())
}
