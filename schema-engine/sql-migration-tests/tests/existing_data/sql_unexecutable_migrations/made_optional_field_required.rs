use quaint::Value;
use sql_migration_tests::test_api::*;
use sql_schema_describer::DefaultValue;

#[test_connector]
fn making_an_optional_field_required_with_data_without_a_default_is_unexecutable(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            name String
            age Int?
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

    let error = format!(
        "Made the column `age` on table `{}` required, but there are 1 existing NULL values.",
        api.normalize_identifier("Test")
    );

    api.schema_push_w_datasource(dm2)
        .send()
        .assert_no_warning()
        .assert_unexecutable(&[error]);

    api.assert_schema()
        .assert_table("Test", |table| table.assert_does_not_have_column("Int"));

    api.dump_table("Test")
        .assert_single_row(|row| row.assert_text_value("id", "abc").assert_text_value("name", "george"));
}

#[test_connector(tags(Sqlite))]
fn making_an_optional_field_required_with_data_with_a_default_works(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            name String
            age Int?
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Test")
        .value("id", "abc")
        .value("name", "george")
        .result_raw();

    api.insert("Test")
        .value("id", "def")
        .value("name", "X Æ A-12")
        .value("age", 7i64)
        .result_raw();

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int @default(84)
        }
    "#;

    api.schema_push_w_datasource(dm2).force(true).send();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("age", |column| {
            column
                .assert_is_required()
                .assert_default(Some(DefaultValue::value(84)))
        })
    });

    let rows = api.dump_table("Test");

    assert_eq!(
        rows.into_iter()
            .map(|row| row.into_iter().collect::<Vec<Value>>())
            .collect::<Vec<_>>(),
        &[
            &[Value::text("abc"), Value::text("george"), Value::int32(84)],
            &[Value::text("def"), Value::text("X Æ A-12"), Value::int32(7)],
        ]
    );
}

// CONFIRMED: this is unexecutable on postgres
// CONFIRMED: all mysql versions except 5.6 will return an error. 5.6 will just insert 0s, which
// seems very wrong, so we should warn against it.
#[test_connector(exclude(Sqlite))]
fn making_an_optional_field_required_with_data_with_a_default_is_unexecutable(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            name String
            age Int?
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Test")
        .value("id", "abc")
        .value("name", "george")
        .result_raw();

    api.insert("Test")
        .value("id", "def")
        .value("name", "X Æ A-12")
        .value("age", 7i64)
        .result_raw();

    let dm2 = r#"
        model Test {
            id String @id
            name String
            age Int @default(84)
        }
    "#;

    let error = format!(
        "Made the column `age` on table `{}` required, but there are 1 existing NULL values.",
        api.normalize_identifier("Test")
    );

    api.schema_push_w_datasource(dm2)
        .force(false)
        .send()
        .assert_unexecutable(&[error])
        .assert_no_warning()
        .assert_no_steps();

    let rows = api.dump_table("Test");

    assert_eq!(
        rows.into_iter()
            .map(|row| row.into_iter().collect::<Vec<Value>>())
            .collect::<Vec<_>>(),
        &[
            &[Value::text("abc"), Value::text("george"), Value::null_int32()],
            &[Value::text("def"), Value::text("X Æ A-12"), Value::int32(7)],
        ]
    );
}

#[test_connector]
fn making_an_optional_field_required_on_an_empty_table_works(api: TestApi) {
    let dm1 = r#"
        model Test {
            id String @id
            name String
            age Int?
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
        .assert_table("Test", |table| table.assert_does_not_have_column("Int"));

    assert!(api.dump_table("Test").is_empty());
}
