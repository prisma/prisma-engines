use migration_engine_tests::sync_test_api::*;

#[test_connector]
fn reset_works(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    api.schema_push(dm).send();

    api.assert_schema().assert_tables_count(1);

    api.insert("Cat").value("id", 1).value("name", "Garfield").result_raw();

    api.reset().send_sync();

    api.assert_schema().assert_tables_count(0);

    api.schema_push(dm).send();

    api.assert_schema().assert_tables_count(1);
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
        .assert_has_table("Cat")
        .assert_has_table("_prisma_migrations");

    api.insert("Cat").value("id", 1).value("name", "Garfield").result_raw();

    api.reset().send_sync();

    api.assert_schema().assert_tables_count(0);

    api.apply_migrations(&dir).send_sync();

    api.assert_schema()
        .assert_tables_count(2)
        .assert_has_table("Cat")
        .assert_has_table("_prisma_migrations");
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
        .assert_has_table("Cat")
        .assert_has_table("_prisma_migrations");

    api.insert("Cat").value("id", 1).value("name", "Garfield").result_raw();

    api.reset().send_sync();

    api.assert_schema().assert_tables_count(0);

    api.diagnose_migration_history(&dir).send_sync();
    api.evaluate_data_loss(&dir, dm.into()).send();
    api.apply_migrations(&dir).send_sync();

    api.assert_schema()
        .assert_tables_count(2)
        .assert_has_table("Cat")
        .assert_has_table("_prisma_migrations");
}
