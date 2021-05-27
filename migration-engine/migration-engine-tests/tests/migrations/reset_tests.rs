use migration_engine_tests::sync_test_api::*;

#[test_connector]
fn reset_works(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    api.schema_push(dm).send_sync();

    api.assert_schema().assert_tables_count(1).unwrap();

    api.insert("Cat").value("id", 1).value("name", "Garfield").result_raw();

    api.reset().send_sync();

    api.assert_schema().assert_tables_count(0).unwrap();

    api.schema_push(dm).send_sync();

    api.assert_schema().assert_tables_count(1).unwrap();
}

#[test_connector]
fn reset_then_apply_with_migrations_directory_works(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let dir = api.create_migrations_directory();
    api.create_migration("0-init", dm, &dir).send_sync();
    api.apply_migrations(&dir).send_sync();

    api.assert_schema()
        .assert_tables_count(2)
        .unwrap()
        .assert_has_table("Cat")
        .unwrap()
        .assert_has_table("_prisma_migrations")
        .unwrap();

    api.insert("Cat").value("id", 1).value("name", "Garfield").result_raw();

    api.reset().send_sync();

    api.assert_schema().assert_tables_count(0).unwrap();

    api.apply_migrations(&dir).send_sync();

    api.assert_schema()
        .assert_tables_count(2)
        .unwrap()
        .assert_has_table("Cat")
        .unwrap()
        .assert_has_table("_prisma_migrations")
        .unwrap();
}

#[test_connector]
fn reset_then_diagnostics_with_migrations_directory_works(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let dir = api.create_migrations_directory();
    api.create_migration("0-init", dm, &dir).send_sync();
    api.apply_migrations(&dir).send_sync();

    api.assert_schema()
        .assert_tables_count(2)
        .unwrap()
        .assert_has_table("Cat")
        .unwrap()
        .assert_has_table("_prisma_migrations")
        .unwrap();

    api.insert("Cat").value("id", 1).value("name", "Garfield").result_raw();

    api.reset().send_sync();

    api.assert_schema().assert_tables_count(0).unwrap();

    api.diagnose_migration_history(&dir).send_sync();
    api.evaluate_data_loss(&dir, dm.into()).send();
    api.apply_migrations(&dir).send_sync();

    api.assert_schema()
        .assert_tables_count(2)
        .unwrap()
        .assert_has_table("Cat")
        .unwrap()
        .assert_has_table("_prisma_migrations")
        .unwrap();
}
