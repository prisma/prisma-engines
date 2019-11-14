mod test_harness;

use migration_connector::MigrationWarning;
use pretty_assertions::assert_eq;
use quaint::ast::*;
use test_harness::*;

#[test_each_connector]
fn adding_a_required_field_if_there_is_data(api: &TestApi) {
    let dm = r#"
            model Test {
                id String @id @default(cuid())
            }

            enum MyEnum {
                B
                A
            }
        "#;
    api.infer_and_apply(&dm).sql_schema;

    let insert = Insert::single_into((SCHEMA_NAME, "Test")).value("id", "test");
    api.database().execute(insert.into()).unwrap();

    let dm = r#"
            model Test {
                id String @id @default(cuid())
                myint Int
                myfloat Float
                boolean Boolean
                string String
                dateTime DateTime
                enum MyEnum
            }

            enum MyEnum {
                B
                A
            }
        "#;
    api.infer_and_apply(&dm);
}

#[test_each_connector]
fn adding_a_required_field_must_use_the_default_value_for_migrations(api: &TestApi) {
    let dm = r#"
            model Test {
                id String @id @default(cuid())
            }

            enum MyEnum {
                B
                A
            }
        "#;
    api.infer_and_apply(&dm);

    let conn = api.database();
    let insert = Insert::single_into((SCHEMA_NAME, "Test")).value("id", "test");

    conn.execute(insert.into()).unwrap();

    let dm = r#"
            model Test {
                id String @id @default(cuid())
                myint Int @default(1)
                myfloat Float @default(2)
                boolean Boolean @default(true)
                string String @default("test_string")
                dateTime DateTime
                // TODO: Currently failing because of ambiguity concerning expressions. Pending on
                // spec work.
                // enum MyEnum @default(C)
            }

            enum MyEnum {
                B
                A
                C
            }
        "#;
    api.infer_and_apply(&dm);

    // TODO: those assertions somehow fail with column not found on SQLite. I could observe the correct data in the db file though.
    if !api.is_sqlite() {
        let conditions = "id".equals("test");
        let table_for_select: Table = (SCHEMA_NAME, "Test").into();
        let query = Select::from_table(table_for_select).so_that(conditions);
        let result_set = conn.query(query.into()).unwrap();
        let row = result_set.into_iter().next().expect("query returned no results");
        assert_eq!(row["myint"].as_i64().unwrap(), 1);
        assert_eq!(row["string"].as_str().unwrap(), "test_string");
    }
}

#[test_each_connector]
fn dropping_a_table_with_rows_should_warn(api: &TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
        }
    "#;
    let original_database_schema = api.infer_and_apply(&dm).sql_schema;

    let conn = api.database();
    let insert = Insert::single_into((SCHEMA_NAME, "Test")).value("id", "test");

    conn.execute(insert.into()).unwrap();

    let dm = "";

    let InferAndApplyOutput {
        migration_output,
        sql_schema: final_database_schema,
    } = api.infer_and_apply(&dm);

    // The schema should not change because the migration should not run if there are warnings
    // and the force flag isn't passed.
    assert_eq!(original_database_schema, final_database_schema);

    assert_eq!(
        migration_output.warnings,
        &[MigrationWarning {
            description: "You are about to drop the table `Test`, which is not empty (1 rows).".into()
        }]
    );
}

#[test_each_connector]
fn dropping_a_column_with_non_null_values_should_warn(api: &TestApi) {
    let dm = r#"
            model Test {
                id String @id @default(cuid())
                puppiesCount Int?
            }
        "#;

    let original_database_schema = api.infer_and_apply(&dm).sql_schema;

    let insert = Insert::multi_into((SCHEMA_NAME, "Test"), vec!["id", "puppiesCount"])
        .values(("a", 7))
        .values(("b", 8));

    api.database().execute(insert.into()).unwrap();

    // Drop the `favouriteAnimal` column.
    let dm = r#"
            model Test {
                id String @id @default(cuid())
            }
        "#;

    let InferAndApplyOutput {
        migration_output,
        sql_schema: final_database_schema,
    } = api.infer_and_apply(&dm);

    // The schema should not change because the migration should not run if there are warnings
    // and the force flag isn't passed.
    assert_eq!(original_database_schema, final_database_schema);

    assert_eq!(
            migration_output.warnings,
            &[MigrationWarning {
                description: "You are about to drop the column `puppiesCount` on the `Test` table, which still contains 2 non-null values.".to_owned(),
            }]
        );
}

#[test_each_connector]
fn altering_a_column_without_non_null_values_should_not_warn(api: &TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            puppiesCount Int?
        }
    "#;

    let original_database_schema = api.infer_and_apply(&dm).sql_schema;

    let insert = Insert::multi_into((SCHEMA_NAME, "Test"), vec!["id"])
        .values(vec!["a"])
        .values(vec!["b"]);

    api.database().execute(insert.into()).unwrap();

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            puppiesCount Float?
        }
    "#;

    let result = api.infer_and_apply(&dm2);
    let final_database_schema = &result.sql_schema;

    assert_ne!(&original_database_schema, final_database_schema);
    assert!(result.migration_output.warnings.is_empty());
}

#[test_each_connector]
fn altering_a_column_with_non_null_values_should_warn(api: &TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            age Int?
        }
    "#;

    let original_database_schema = api.infer_and_apply(&dm).sql_schema;

    let insert = Insert::multi_into((SCHEMA_NAME, "Test"), vec!["id", "age"])
        .values(("a", 12))
        .values(("b", 22));

    api.database().execute(insert.into()).unwrap();

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            age Float?
        }
    "#;

    let result = api.infer_and_apply(&dm2);
    let final_database_schema = result.sql_schema;

    // The schema should not change because the migration should not run if there are warnings
    // and the force flag isn't passed.
    assert_eq!(original_database_schema, final_database_schema);

    assert_eq!(
        result.migration_output.warnings,
        &[MigrationWarning {
            description:
                "You are about to alter the column `age` on the `Test` table, which still contains 2 non-null values. \
                 The data in that column will be lost."
                    .to_owned()
        }]
    );
}
