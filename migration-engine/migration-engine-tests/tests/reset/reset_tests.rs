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

#[test_each_connector]
async fn reset_then_apply_with_migrations_directory_works(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let dir = api.create_migrations_directory()?;
    api.create_migration("0-init", dm, &dir).send().await?;
    api.apply_migrations(&dir).send().await?;

    api.assert_schema()
        .await?
        .assert_tables_count(2)?
        .assert_has_table("Cat")?
        .assert_has_table("_prisma_migrations")?;

    api.insert("Cat")
        .value("id", 1)
        .value("name", "Garfield")
        .result_raw()
        .await?;

    api.reset().send().await?;

    api.assert_schema().await?.assert_tables_count(0)?;

    api.apply_migrations(&dir).send().await?;

    api.assert_schema()
        .await?
        .assert_tables_count(2)?
        .assert_has_table("Cat")?
        .assert_has_table("_prisma_migrations")?;

    Ok(())
}

#[test_each_connector]
async fn reset_then_diagnostics_with_migrations_directory_works(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let dir = api.create_migrations_directory()?;
    api.create_migration("0-init", dm, &dir).send().await?;
    api.apply_migrations(&dir).send().await?;

    api.assert_schema()
        .await?
        .assert_tables_count(2)?
        .assert_has_table("Cat")?
        .assert_has_table("_prisma_migrations")?;

    api.insert("Cat")
        .value("id", 1)
        .value("name", "Garfield")
        .result_raw()
        .await?;

    api.reset().send().await?;

    api.assert_schema().await?.assert_tables_count(0)?;

    api.diagnose_migration_history(&dir).send().await?;
    api.evaluate_data_loss(&dir, dm).send().await?;
    api.apply_migrations(&dir).send().await?;

    api.assert_schema()
        .await?
        .assert_tables_count(2)?
        .assert_has_table("Cat")?
        .assert_has_table("_prisma_migrations")?;

    Ok(())
}
