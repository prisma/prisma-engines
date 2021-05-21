use migration_engine_tests::sync_test_api::*;

#[test_connector]
fn a_model_can_be_removed(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = r#"
        model User {
            id   Int     @id @default(autoincrement())
            name String?
            Post Post[]
        }

        model Post {
            id     Int    @id @default(autoincrement())
            title  String
            User   User   @relation(fields: [userId], references: [id])
            userId Int
        }
    "#;

    api.create_migration("initial", dm1, &directory).send_sync();

    let dm2 = r#"
        model User {
            id   Int     @id @default(autoincrement())
            name String?
        }
    "#;

    api.create_migration("second-migration", dm2, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let output = api.diagnose_migration_history(&directory).send_sync().into_output();

    assert!(output.is_empty());
}
