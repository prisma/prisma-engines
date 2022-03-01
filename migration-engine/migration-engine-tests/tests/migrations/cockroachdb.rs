use migration_engine_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
fn soft_resets_work_on_cockroachdb(mut api: TestApi) {
    let initial = r#"
        CREATE TABLE "Cat" ( id TEXT PRIMARY KEY, name TEXT, meowmeow BOOLEAN );
        CREATE VIEW "catcat" AS SELECT name, meowmeow FROM "Cat" LIMIT 2;
    "#;

    api.raw_cmd(&initial);
    api.assert_schema().assert_tables_count(1).assert_has_table("Cat");
    api.reset().soft(true).send_sync();
    api.assert_schema().assert_tables_count(0);
}
