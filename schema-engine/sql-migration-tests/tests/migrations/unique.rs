mod mssql;

use sql_migration_tests::test_api::*;

#[test_connector]
fn adding_a_new_unique_field_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String @unique
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |index| index.assert_is_unique())
    });
}

#[test_connector]
fn adding_new_fields_with_multi_column_unique_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField String

            @@unique([field, secondField])
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    });
}

#[test_connector]
fn unique_in_conjunction_with_custom_column_name_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String @unique @map("custom_field_name")
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_index_on_columns(&["custom_field_name"], |idx| idx.assert_is_unique())
    });
}

#[test_connector]
fn multi_column_unique_in_conjunction_with_custom_column_name_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            field String @map("custom_field_name")
            secondField String @map("second_custom_field_name")

            @@unique([field, secondField])
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_index_on_columns(&["custom_field_name", "second_custom_field_name"], |idx| {
            idx.assert_is_unique()
        })
    });
}

#[test_connector]
fn removing_an_existing_unique_field_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id    Int    @id
            field String @unique
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id    Int    @id
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("A", |t| {
        t.assert_indexes_count(0)
            .assert_columns_count(1)
            .assert_has_column("id")
    });
}

#[test_connector]
fn adding_unique_to_an_existing_field_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id    Int    @id
            field String
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema()
        .assert_table("A", |table| table.assert_indexes_count(0));

    let dm2 = r#"
        model A {
            id    Int    @id
            field String @unique
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_executable()
        .assert_warnings(&["A unique constraint covering the columns `[field]` on the table `A` will be added. If there are existing duplicate values, this will fail.".into()])
        .assert_has_executed_steps();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_indexes_count(1)
            .assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    });
}

#[test_connector]
fn removing_unique_from_an_existing_field_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id    Int    @id
            field String @unique
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id    Int    @id
            field String
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();
    api.assert_schema().assert_table("A", |t| t.assert_indexes_count(0));
}

#[test_connector]
fn unique_is_allowed_on_an_id_field(api: TestApi) {
    let dm1 = r#"
        model A {
            id    Int    @id @unique
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("A", |t| {
        t.assert_index_on_columns(&["id"], |idx| idx.assert_is_unique())
    });
}
