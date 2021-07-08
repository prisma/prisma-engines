use migration_engine_tests::sync_test_api::*;
use quaint::Value;
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
        .send()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.schema_push(&dm).send().assert_green_bang().assert_no_steps();
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
        .send()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.schema_push(dm).send().assert_green_bang().assert_no_steps();
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
        .send()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.schema_push(&dm).send().assert_green_bang().assert_no_steps();
}

#[test_connector]
fn float_to_decimal_works(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id String @id
            meowFrequency Float
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();

    api.assert_schema().assert_table("Cat", |table| {
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
        .send()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Cat", |table| {
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

    api.schema_push(&dm1).send().assert_green_bang();

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("meowFrequency", |col| col.assert_type_family(ColumnTypeFamily::Decimal))
    });

    let dm2 = r#"
        model Cat {
            id String @id
            meowFrequency Float
        }
    "#;

    api.schema_push(dm2)
        .send()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Cat", |table| {
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

    api.schema_push(&dm1).send().assert_green_bang();

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("meowData", |col| col.assert_type_is_bytes())
    });

    let dm2 = r#"
        model Cat {
            id String @id
            meowData String
        }
    "#;

    api.schema_push(dm2)
        .send()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Cat", |table| {
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

    api.schema_push(&dm1).send().assert_green_bang();

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("meowData", |col| col.assert_type_is_bytes())
    });

    let dm2 = r#"
        model Cat {
            id String @id
            meowData String
        }
    "#;

    api.schema_push(dm2)
        .send()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Cat", |table| {
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

    api.schema_push(dm1).send().assert_green_bang();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("decFloat", |col| col.assert_type_is_decimal().assert_is_required())
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
        .send()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("decFloat", |col| col.assert_type_is_decimal().assert_is_list())
    });

    api.schema_push(dm1).send().assert_green_bang();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("decFloat", |col| col.assert_type_is_decimal().assert_is_required())
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

    api.schema_push(&dm1).send().assert_green_bang();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("bytesCol", |col| col.assert_type_is_bytes().assert_is_required())
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
        .send()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("bytesCol", |col| col.assert_type_is_bytes().assert_is_list())
    });

    api.schema_push(&dm1).send().assert_green_bang();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("bytesCol", |col| col.assert_type_is_bytes().assert_is_required())
    });
}

#[test_connector(tags(Mssql))]
fn a_table_recreation_with_noncastable_columns_should_trigger_warnings(api: TestApi) {
    let dm1 = r#"
        model Blog {
            id Int @id @default(autoincrement())
            title String
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();

    // Removing autoincrement requires us to recreate the table.
    let dm2 = r#"
        model Blog {
            id Int @id
            title Float
        }
    "#;

    api.insert("Blog").value("title", "3.14").result_raw();

    api.schema_push(dm2)
        .send()
        .assert_warnings(&["You are about to alter the column `title` on the `Blog` table, which contains 1 non-null values. The data in that column will be cast from `String` to `Float`.".into()]);
}

#[test_connector(tags(Postgres))]
fn a_column_recreation_with_non_castable_type_change_should_trigger_warnings(api: TestApi) {
    let dm1 = r#"
        datasource pg {
            provider = "postgres"
            url = env("DBURL")
        }

        model Blog {
            id      Int @id
            float   Float
        }
    "#;

    api.schema_push(dm1).send().assert_green_bang();
    let insert = quaint::ast::Insert::single_into((api.schema_name(), "Blog"))
        .value("id", 1)
        .value("float", Value::double(7.5));

    api.query(insert.into());
    let dm2 = r#"
        datasource pg {
            provider = "postgres"
            url = env("DBURL")
        }

        model Blog {
            id      Int @id
            float   DateTime
        }
    "#;

    api.schema_push(dm2)
        .send()
        .assert_unexecutable(&["Changed the type of `float` on the `Blog` table. No cast exists, the column would be dropped and recreated, which cannot be done since the column is required and there is data in the table.".into()]);
}
