use migration_engine_tests::sync_test_api::*;
use sql_schema_describer::ColumnTypeFamily;

#[test_connector]
fn bytes_columns_are_idempotent(api: TestApi) {
    let dm = format!(
        r#"
        {datasource}

        model Cat {{
            id String @id
            chipData Bytes
        }}
    "#,
        datasource = api.datasource_block()
    );

    api.schema_push(&dm)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.schema_push(&dm).send_sync().assert_green_bang().assert_no_steps();
}

#[test_connector]
fn float_columns_are_idempotent(api: TestApi) {
    let dm = r#"
        model Cat {
            id String @id
            meowFrequency Float
        }
    "#;

    api.schema_push(dm)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.schema_push(dm).send_sync().assert_green_bang().assert_no_steps();
}

#[test_connector]
fn decimal_columns_are_idempotent(api: TestApi) {
    let dm = format!(
        r#"
        {datasource}

        model Cat {{
            id String @id
            meowFrequency Decimal
        }}
        "#,
        datasource = api.datasource_block()
    );

    api.schema_push(&dm)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.schema_push(&dm).send_sync().assert_green_bang().assert_no_steps();
}

#[test_connector]
fn float_to_decimal_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id String @id
            meowFrequency Float
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table.assert_column("meowFrequency", |col| col.assert_type_family(ColumnTypeFamily::Float))
    });

    let dm2 = format!(
        r#"
        {datasource}

        model Cat {{
            id String @id
            meowFrequency Decimal
        }}
    "#,
        datasource = api.datasource_block()
    );

    api.schema_push(&dm2)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table.assert_column("meowFrequency", |col| col.assert_type_family(ColumnTypeFamily::Decimal))
    });
}

#[test_connector]
fn decimal_to_float_works(api: TestApi) {
    let dm1 = format!(
        r#"
        {datasource}

        model Cat {{
            id String @id
            meowFrequency Decimal
        }}
    "#,
        datasource = api.datasource_block()
    );

    api.schema_push(&dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table.assert_column("meowFrequency", |col| col.assert_type_family(ColumnTypeFamily::Decimal))
    });

    let dm2 = r#"
        model Cat {
            id String @id
            meowFrequency Float
        }
    "#;

    api.schema_push(dm2)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table.assert_column("meowFrequency", |col| col.assert_type_family(ColumnTypeFamily::Float))
    });
}

#[test_connector]
fn bytes_to_string_works(api: TestApi) {
    let dm1 = format!(
        r#"
        {datasource}

        model Cat {{
            id String @id
            meowData Bytes
        }}
    "#,
        datasource = api.datasource_block()
    );

    api.schema_push(&dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table.assert_column("meowData", |col| col.assert_type_is_bytes())
    });

    let dm2 = r#"
        model Cat {
            id String @id
            meowData String
        }
    "#;

    api.schema_push(dm2)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table.assert_column("meowData", |col| col.assert_type_is_string())
    });
}

#[test_connector]
fn string_to_bytes_works(api: TestApi) {
    let dm1 = format!(
        r#"
        {datasource}

        model Cat {{
            id String @id
            meowData Bytes
        }}
    "#,
        datasource = api.datasource_block()
    );

    api.schema_push(&dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table.assert_column("meowData", |col| col.assert_type_is_bytes())
    });

    let dm2 = r#"
        model Cat {
            id String @id
            meowData String
        }
    "#;

    api.schema_push(dm2)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table_bang("Cat", |table| {
        table.assert_column("meowData", |col| col.assert_type_is_string())
    });
}

#[test_connector(capabilities(ScalarLists))]
fn decimal_to_decimal_array_works(api: TestApi) {
    let dm1 = r#"
        model Test {
            id       String    @id @default(cuid())
            decFloat Decimal
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Test", |table| {
        table.assert_column("decFloat", |col| col.assert_type_is_decimal()?.assert_is_required())
    });

    let dm2 = format!(
        r#"
        {}

        model Test {{
            id       String    @id @default(cuid())
            decFloat Decimal[]
        }}
        "#,
        api.datasource_block()
    );

    api.schema_push(dm2)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table_bang("Test", |table| {
        table.assert_column("decFloat", |col| col.assert_type_is_decimal()?.assert_is_list())
    });

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Test", |table| {
        table.assert_column("decFloat", |col| col.assert_type_is_decimal()?.assert_is_required())
    });
}

#[test_connector(capabilities(ScalarLists))]
fn bytes_to_bytes_array_works(api: TestApi) {
    let dm1 = format!(
        r#"
            {datasource}

            model Test {{
                id       String    @id @default(cuid())
                bytesCol Bytes
            }}
        "#,
        datasource = api.datasource_block()
    );

    api.schema_push(&dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Test", |table| {
        table.assert_column("bytesCol", |col| col.assert_type_is_bytes()?.assert_is_required())
    });

    let dm2 = format!(
        r#"
            {datasource}

            model Test {{
                id       String    @id @default(cuid())
                bytesCol Bytes[]
            }}
        "#,
        datasource = api.datasource_block()
    );

    api.schema_push(dm2)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table_bang("Test", |table| {
        table.assert_column("bytesCol", |col| col.assert_type_is_bytes()?.assert_is_list())
    });

    api.schema_push(&dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("Test", |table| {
        table.assert_column("bytesCol", |col| col.assert_type_is_bytes()?.assert_is_required())
    });
}
