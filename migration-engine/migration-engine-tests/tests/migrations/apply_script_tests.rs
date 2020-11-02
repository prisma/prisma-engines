use crate::*;
use pretty_assertions::assert_eq;

#[test_each_connector]
async fn apply_script_applies_the_script_without_touching_migrations_persistence(api: &TestApi) -> TestResult {
    let dir = api.create_migrations_directory()?;

    let dm = r#"
        model Test {
            id Int @id
        }

        model test2 {
            id Int @id
        }
    "#;

    let out = api
        .create_migration("initial", dm, &dir)
        .send()
        .await?
        .assert_migration_directories_count(1)?;

    api.apply_migrations(&dir).send().await?;

    api.apply_script("DROP TABLE test2").await?;

    // There is no new migration in the folder.
    out.assert_migration_directories_count(1)?;

    // There is no new migration in the table.
    let migrations = api.imperative_migration_persistence().list_migrations().await?.unwrap();
    assert_eq!(migrations.len(), 1);

    // The script was applied
    api.assert_schema()
        .await?
        .assert_tables_count(2)?
        .assert_has_table("_prisma_migrations")?
        .assert_has_table("Test")?;

    Ok(())
}
