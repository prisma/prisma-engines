use sql_migration_tests::test_api::*;

#[test_connector]
fn adding_a_required_field_to_an_existing_table_with_data_without_a_default_is_unexecutable(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Test")
        .value("id", "abc")
        .value("name", "george")
        .result_raw();

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(false)
        .send()
        .assert_no_warning()
        .assert_unexecutable(&["Added the required column `age` to the `Test` table without a default value. There are 1 rows in this table, it is not possible to execute this step.".to_string()]);

    api.dump_table("Test")
        .assert_single_row(|row| row.assert_text_value("id", "abc").assert_text_value("name", "george"));
}

#[test_connector]
fn adding_a_required_field_with_prisma_level_default_works(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            age Int
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Test").value("id", "abc").value("age", 100).result_raw();

    let dm2 = r#"
        model Test {
            id String @id
            age Int
            name String @default(cuid())
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(false)
        .send()
        .assert_no_warning()
        .assert_unexecutable(&["The required column `name` was added to the `Test` table with a prisma-level default value. There are 1 rows in this table, it is not possible to execute this step. Please add this column as optional, then populate it before making it required.".into()]);

    api.dump_table("Test")
        .assert_single_row(|row| row.assert_text_value("id", "abc").assert_int_value("age", 100));
}

#[test_connector]
fn adding_a_required_field_with_a_default_to_an_existing_table_works(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Test")
        .value("id", "abc")
        .value("name", "george")
        .result_raw();

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int @default(45)
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.dump_table("Test").assert_single_row(|row| {
        row.assert_text_value("id", "abc")
            .assert_text_value("name", "george")
            .assert_int_value("age", 45)
    });
}

#[test_connector]
fn adding_a_required_field_without_default_to_an_existing_table_without_data_works(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema()
        .assert_table("Test", |table| table.assert_has_column("age"));
}
