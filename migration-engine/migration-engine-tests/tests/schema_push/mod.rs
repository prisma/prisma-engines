use migration_engine_tests::sql::*;
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
}
"#;

#[test_each_connector(log = "sql_schema_describer=info,debug")]
async fn schema_push_happy_path(api: &TestApi) -> TestResult {
    api.schema_push(SCHEMA)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;

    api.assert_schema()
        .await?
        .assert_table("Cat", |table| {
            table.assert_column("boxId", |col| col.assert_type_family(ColumnTypeFamily::Int))
        })?
        .assert_table("Box", |table| {
            table.assert_column("material", |col| col.assert_type_family(ColumnTypeFamily::String))
        })?;

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
    }
    "#;

    api.schema_push(dm2)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;

    api.assert_schema()
        .await?
        .assert_table("Cat", |table| {
            table.assert_column("boxId", |col| col.assert_type_family(ColumnTypeFamily::Int))
        })?
        .assert_table("Box", |table| {
            table
                .assert_columns_count(3)?
                .assert_column("texture", |col| col.assert_type_family(ColumnTypeFamily::String))
        })?;

    Ok(())
}

#[test_each_connector]
async fn schema_push_warns_about_destructive_changes(api: &TestApi) -> TestResult {
    api.schema_push(SCHEMA)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;

    api.insert("Box")
        .value("id", 1)
        .value("material", "cardboard")
        .result_raw()
        .await?;

    let dm2 = r#"
        model Cat {
            id Int @id
        }
    "#;

    let expected_warning = "You are about to drop the `Box` table, which is not empty (1 rows).";

    api.schema_push(dm2)
        .send()
        .await?
        .assert_warnings(&[expected_warning.into()])?
        .assert_no_steps()?;

    api.schema_push(dm2)
        .force(true)
        .send()
        .await?
        .assert_warnings(&[expected_warning.into()])?
        .assert_has_executed_steps()?;

    Ok(())
}

#[test_each_connector]
async fn schema_push_with_an_unexecutable_migration_returns_a_message_and_aborts(api: &TestApi) -> TestResult {
    api.schema_push(SCHEMA)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;

    api.insert("Box")
        .value("id", 1)
        .value("material", "cardboard")
        .result_raw()
        .await?;

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
        }
    "#;

    api.schema_push(dm2)
        .send()
        .await?
        .assert_unexecutable(&["Added the required column `volumeCm3` to the `Box` table without a default value. There are 1 rows in this table, it is not possible to execute this migration.".into()])?
        .assert_no_steps()?;

    Ok(())
}

#[test_each_connector]
async fn indexes_and_unique_constraints_on_the_same_field_do_not_collide(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            id     Int    @id @default(autoincrement())
            email  String @unique
            name   String

            @@index([email])
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    Ok(())
}

#[test_each_connector]
async fn multi_column_indexes_and_unique_constraints_on_the_same_fields_do_not_collide(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            id     Int    @id @default(autoincrement())
            email  String
            name   String

            @@index([email, name])
            @@unique([email, name])
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    Ok(())
}
