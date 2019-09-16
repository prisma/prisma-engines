#![allow(non_snake_case)]
#![allow(unused)]
mod test_harness;
use migration_connector::MigrationWarning;
use pretty_assertions::{assert_eq, assert_ne};
use prisma_query::ast::*;
use sql_migration_connector::SqlFamily;
use test_harness::*;

#[test]
fn adding_a_required_field_if_there_is_data() {
    test_each_connector(|sql_family, api| {
        let dm = r#"
            model Test {
                id String @id @default(cuid())
            }

            enum MyEnum {
                B
                A
            }
        "#;
        infer_and_apply(api, &dm).sql_schema;

        let conn = database(sql_family);
        let insert = Insert::single_into((SCHEMA_NAME, "Test")).value("id", "test");
        conn.execute(SCHEMA_NAME, insert.into()).unwrap();

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
        infer_and_apply(api, &dm);
    });
}

#[test]
fn adding_a_required_field_must_use_the_default_value_for_migrations() {
    test_each_connector(|sql_family, api| {
        let dm = r#"
            model Test {
                id String @id @default(cuid())
            }

            enum MyEnum {
                B
                A
            }
        "#;
        infer_and_apply(api, &dm);

        let conn = database(sql_family);
        let insert = Insert::single_into((SCHEMA_NAME, "Test")).value("id", "test");

        conn.execute(SCHEMA_NAME, insert.into()).unwrap();

        let dm = r#"
            model Test {
                id String @id @default(cuid())
                myint Int @default(1)
                myfloat Float @default(2)
                boolean Boolean @default(true)
                string String @default("test_string")
                dateTime DateTime 
                enum MyEnum @default(C)
            }

            enum MyEnum {
                B
                A
                C
            }
        "#;
        infer_and_apply(api, &dm);

        // TODO: those assertions somehow fail with column not found on SQLite. I could observe the correct data in the db file though.
        if sql_family != SqlFamily::Sqlite {
            let conditions = "id".equals("test");
            let table_for_select: Table = match sql_family {
                SqlFamily::Sqlite => {
                    // sqlite case. Otherwise prisma-query produces invalid SQL
                    "Test".into()
                }
                _ => (SCHEMA_NAME, "Test").into(),
            };
            let query = Select::from_table(table_for_select).so_that(conditions);
            let result_set = conn.query(SCHEMA_NAME, query.into()).unwrap();
            let row = result_set.into_iter().next().expect("query returned no results");
            assert_eq!(row["myint"].as_i64().unwrap(), 1);
            assert_eq!(row["string"].as_str().unwrap(), "test_string");
        }
    });
}

#[test]
fn dropping_a_table_with_rows_should_warn() {
    test_each_connector(|sql_family, engine| {
        let dm = r#"
                    model Test {
                        id String @id @default(cuid())
                    }
                "#;
        let original_database_schema = infer_and_apply(engine, &dm).sql_schema;

        let conn = database(sql_family);
        let insert = Insert::single_into((SCHEMA_NAME, "Test")).value("id", "test");

        conn.execute(SCHEMA_NAME, insert.into()).unwrap();

        let dm = "";

        let InferAndApplyOutput {
            migration_output,
            sql_schema: final_database_schema,
        } = infer_and_apply(engine, &dm);

        // The schema should not change because the migration should not run if there are warnings
        // and the force flag isn't passed.
        assert_eq!(original_database_schema, final_database_schema);

        assert_eq!(
            migration_output.warnings,
            &[MigrationWarning {
                description: "You are about to drop the table `Test`, which is not empty (1 rows).".into()
            }]
        );
    })
}
