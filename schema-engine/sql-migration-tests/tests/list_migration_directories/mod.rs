use sql_migration_tests::test_api::*;

#[test_connector]
fn list_migration_directories_with_an_empty_migrations_folder_works(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    api.list_migration_directories(&migrations_directory)
        .send()
        .assert_listed_directories(&[]);
}

#[test_connector]
fn listing_a_single_migration_name_should_work(api: TestApi) {
    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id Int @id
            name String
        }
    "#,
    );

    let migrations_directory = api.create_migrations_directory();

    api.create_migration("init", &dm, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["init"]);

    api.list_migration_directories(&migrations_directory)
        .send()
        .assert_listed_directories(&["init"]);
}
