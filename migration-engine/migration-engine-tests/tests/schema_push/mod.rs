use migration_engine_tests::sync_test_api::*;
use sql_schema_describer::ColumnTypeFamily;

const SCHEMA: &str = r#"
model Cat {
    id Int @id
    boxId Int?
    box Box? @relation(fields: [boxId], references: [id])
}

model Box {
    id Int @id
    material String
    cats     Cat[]
}
"#;

#[test_connector]
fn schema_push_happy_path(api: TestApi) {
    api.schema_push(SCHEMA)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema()
        .assert_table_bang("Cat", |table| {
            table.assert_column("boxId", |col| col.assert_type_family(ColumnTypeFamily::Int))
        })
        .assert_table_bang("Box", |table| {
            table.assert_column("material", |col| col.assert_type_family(ColumnTypeFamily::String))
        });

    let dm2 = r#"
    model Cat {
        id Int @id
        boxId Int?
        residence Box? @relation(fields: [boxId], references: [id])
    }

    model Box {
        id Int @id
        texture String
        waterProof Boolean
        cats       Cat[]
    }
    "#;

    api.schema_push(dm2)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.assert_schema()
        .assert_table_bang("Cat", |table| {
            table.assert_column("boxId", |col| col.assert_type_family(ColumnTypeFamily::Int))
        })
        .assert_table_bang("Box", |table| {
            table
                .assert_columns_count(3)?
                .assert_column("texture", |col| col.assert_type_family(ColumnTypeFamily::String))
        });
}

#[test_connector]
fn schema_push_warns_about_destructive_changes(api: TestApi) {
    api.schema_push(SCHEMA)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.insert("Box")
        .value("id", 1)
        .value("material", "cardboard")
        .result_raw();

    let dm2 = r#"
        model Cat {
            id Int @id
        }
    "#;

    let expected_warning = format!(
        "You are about to drop the `{}` table, which is not empty (1 rows).",
        api.normalize_identifier("Box")
    );

    api.schema_push(dm2)
        .send_sync()
        .assert_warnings(&[expected_warning.as_str().into()])
        .assert_no_steps();

    api.schema_push(dm2)
        .force(true)
        .send_sync()
        .assert_warnings(&[expected_warning.as_str().into()])
        .assert_has_executed_steps();
}

#[test_connector]
fn schema_push_with_an_unexecutable_migration_returns_a_message_and_aborts(api: TestApi) {
    api.schema_push(SCHEMA)
        .send_sync()
        .assert_green_bang()
        .assert_has_executed_steps();

    api.insert("Box")
        .value("id", 1)
        .value("material", "cardboard")
        .result_raw();

    let dm2 = r#"
        model Cat {
            id Int @id
            boxId Int?
            box Box? @relation(fields: [boxId], references: [id])
        }

        model Box {
            id Int @id
            material String
            volumeCm3 Int
            cats      Cat[]
        }
    "#;

    api.schema_push(dm2)
        .send_sync()
        .assert_unexecutable(&["Added the required column `volumeCm3` to the `Box` table without a default value. There are 1 rows in this table, it is not possible to execute this step.".into()])
        .assert_no_steps();
}

#[test_connector]
fn indexes_and_unique_constraints_on_the_same_field_do_not_collide(api: TestApi) {
    let dm = r#"
        model User {
            id     Int    @id @default(autoincrement())
            email  String @unique
            name   String

            @@index([email])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();
}

#[test_connector]
fn multi_column_indexes_and_unique_constraints_on_the_same_fields_do_not_collide(api: TestApi) {
    let dm = r#"
        model User {
            id     Int    @id @default(autoincrement())
            email  String
            name   String

            @@index([email, name])
            @@unique([email, name])
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();
}
