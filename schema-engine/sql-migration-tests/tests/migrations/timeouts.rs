use sql_migration_tests::test_api::*;

#[test_connector(
    tags(Postgres),
    ignore = "This test intentionally runs for more than a minute, we don't want it in regular runs."
)]
fn migrations_can_last_more_than_a_minute_and_succeed(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();
    let schema = api.datasource_block().to_string();

    let long_migration_sql = r#"
        SELECT pg_sleep(65);
    "#;

    let short_migration_sql = r#"
        CREATE TABLE cat ( id INTEGER PRIMARY KEY )
    "#;

    api.create_migration("01long_migration", &schema, &migrations_directory)
        .draft(true)
        .send_sync()
        .modify_migration(|migration| {
            migration.clear();
            migration.push_str(long_migration_sql)
        });

    api.create_migration("02short_migration", &schema, &migrations_directory)
        .draft(true)
        .send_sync()
        .modify_migration(|migration| {
            migration.clear();
            migration.push_str(short_migration_sql);
        });

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["01long_migration", "02short_migration"]);

    api.assert_schema().assert_has_table("cat");
}
