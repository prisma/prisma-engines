use sql_migration_tests::test_api::*;

#[test_connector(tags(KingbaseOracle))]
fn oracle_evaluate_data_loss_warns_before_dropping_a_non_empty_table(api: TestApi) {
    let initial_schema = api.datamodel_with_provider(
        r#"
        model Account {
            id   Int    @id
            name String
        }
    "#,
    );

    let directory = api.create_migrations_directory();
    api.create_migration("initial", &initial_schema, &directory).send_sync();
    api.apply_migrations(&directory).send_sync();

    api.insert("Account")
        .value("id", 1)
        .value("name", "kingbase")
        .result_raw();

    let warning = format!(
        "You are about to drop the `{}` table, which is not empty (1 rows).",
        api.normalize_identifier("Account")
    );

    api.evaluate_data_loss(&directory, api.datamodel_with_provider(""))
        .send()
        .assert_warnings(&[warning.into()])
        .assert_unexecutable(&[])
        .assert_steps_count(1);
}
