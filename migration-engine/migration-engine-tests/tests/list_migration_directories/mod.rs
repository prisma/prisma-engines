use crate::*;

#[test_each_connector]
async fn list_migration_directories_with_an_empty_migrations_folder_works(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;

    api.list_migration_directories(&migrations_directory)
        .send()
        .await?
        .assert_listed_directories(&[])?;

    Ok(())
}

#[test_each_connector]
async fn listing_a_single_migration_name_should_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

    api.create_migration("init", dm, &migrations_directory).send().await?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&["init"])?;

    api.list_migration_directories(&migrations_directory)
        .send()
        .await?
        .assert_listed_directories(&["init"])?;

    Ok(())
}
