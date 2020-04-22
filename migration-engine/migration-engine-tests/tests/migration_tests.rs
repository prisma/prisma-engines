mod migrations;

use migration_engine_tests::sql::*;
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use quaint::prelude::SqlFamily;
use sql_migration_connector::{AlterIndex, CreateIndex, DropIndex, SqlMigrationStep};
use sql_schema_describer::*;

#[test_each_connector]
async fn adding_a_scalar_field_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            int Int
            float Float
            boolean Boolean
            string String
            dateTime DateTime
            enum MyEnum
        }

        enum MyEnum {
            A
            B
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table
            .assert_columns_count(7)?
            .assert_column("int", |c| {
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::Int)
            })?
            .assert_column("float", |c| {
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::Float)
            })?
            .assert_column("boolean", |c| {
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::Boolean)
            })?
            .assert_column("string", |c| {
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::String)
            })?
            .assert_column("dateTime", |c| {
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::DateTime)
            })?
            .assert_column("enum", |c| match api.sql_family() {
                SqlFamily::Postgres => c
                    .assert_is_required()?
                    .assert_type_family(ColumnTypeFamily::Enum("MyEnum".to_owned())),
                SqlFamily::Mysql => c
                    .assert_is_required()?
                    .assert_type_family(ColumnTypeFamily::Enum("Test_enum".to_owned())),
                _ => c.assert_is_required()?.assert_type_is_string(),
            })
    })?;

    Ok(())
}

#[test_each_connector(capabilities("json"), ignore("mysql_5_6"))]
async fn json_fields_can_be_created(api: &TestApi) -> TestResult {
    let dm = format!(
        r#"
            {}

            model Test {{
                id String @id @default(cuid())
                javaScriptObjectNotation Json
            }}
        "#,
        api.datasource()
    );

    api.infer_apply(&dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table.assert_column("javaScriptObjectNotation", |c| {
            if api.is_mariadb() {
                // JSON is an alias for LONGTEXT on MariaDB - https://mariadb.com/kb/en/json-data-type/
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::String)
            } else {
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::Json)
            }
        })
    })?;

    api.infer(&dm).send_assert().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

#[test_each_connector]
async fn adding_an_optional_field_must_work(api: &TestApi) -> TestResult {
    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            field String?
        }
    "#;

    api.infer_apply(dm2).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("Test", |table| {
        table.assert_column("field", |column| column.assert_default(None)?.assert_is_nullable())
    })?;

    Ok(())
}

#[test_each_connector]
async fn adding_an_id_field_with_a_special_name_must_work(api: &TestApi) {
    let dm2 = r#"
            model Test {
                specialName String @id @default(cuid())
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let column = result.table_bang("Test").column("specialName");
    assert_eq!(column.is_some(), true);
}

#[test_each_connector(ignore("sqlite"))]
async fn adding_an_id_field_of_type_int_must_work(api: &TestApi) {
    let dm2 = r#"
        model Test {
            myId Int @id
            text String
        }
    "#;

    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let column = result.table_bang("Test").column_bang("myId");

    assert_eq!(column.auto_increment, false);
}

#[test_each_connector(tags("sqlite"))]
async fn adding_an_id_field_of_type_int_must_work_for_sqlite(api: &TestApi) {
    let dm2 = r#"
        model Test {
            myId Int @id
            text String
        }
    "#;

    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let column = result.table_bang("Test").column_bang("myId");

    assert_eq!(column.auto_increment, true);
}

#[test_each_connector]
async fn adding_an_id_field_of_type_int_with_autoincrement_must_work(api: &TestApi) {
    let dm2 = r#"
        model Test {
            myId Int @id @default(autoincrement())
            text String
        }
    "#;

    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let column = result.table_bang("Test").column_bang("myId");

    match api.sql_family() {
        SqlFamily::Postgres => {
            let sequence = result.get_sequence("Test_myId_seq").expect("sequence must exist");
            let default = column.default.as_ref().expect("Must have nextval default");
            assert_eq!(
                DefaultValue::SEQUENCE(format!("nextval('\"{}\"'::regclass)", sequence.name)),
                *default
            );
        }
        _ => assert_eq!(column.auto_increment, true),
    }
}

#[test_each_connector]
async fn removing_a_scalar_field_must_work(api: &TestApi) {
    let dm1 = r#"
            model Test {
                id String @id @default(cuid())
                field String
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let column1 = result.table_bang("Test").column("field");
    assert_eq!(column1.is_some(), true);

    let dm2 = r#"
            model Test {
                id String @id @default(cuid())
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let column2 = result.table_bang("Test").column("field");
    assert_eq!(column2.is_some(), false);
}

#[test_each_connector]
async fn can_handle_reserved_sql_keywords_for_model_name(api: &TestApi) {
    let dm1 = r#"
            model Group {
                id String @id @default(cuid())
                field String
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let column = result.table_bang("Group").column_bang("field");
    assert_eq!(column.tpe.family, ColumnTypeFamily::String);

    let dm2 = r#"
            model Group {
                id String @id @default(cuid())
                field Int
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let column = result.table_bang("Group").column_bang("field");
    assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
}

#[test_each_connector]
async fn can_handle_reserved_sql_keywords_for_field_name(api: &TestApi) {
    let dm1 = r#"
            model Test {
                id String @id @default(cuid())
                Group String
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let column = result.table_bang("Test").column_bang("Group");
    assert_eq!(column.tpe.family, ColumnTypeFamily::String);

    let dm2 = r#"
            model Test {
                id String @id @default(cuid())
                Group Int
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let column = result.table_bang("Test").column_bang("Group");
    assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
}

#[test_each_connector]
async fn update_type_of_scalar_field_must_work(api: &TestApi) {
    let dm1 = r#"
            model Test {
                id String @id @default(cuid())
                field String
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let column1 = result.table_bang("Test").column_bang("field");
    assert_eq!(column1.tpe.family, ColumnTypeFamily::String);

    let dm2 = r#"
            model Test {
                id String @id @default(cuid())
                field Int
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let column2 = result.table_bang("Test").column_bang("field");
    assert_eq!(column2.tpe.family, ColumnTypeFamily::Int);
}

#[test_each_connector]
async fn changing_the_type_of_an_id_field_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                b_id Int
                b  B   @relation(fields: [b_id], references: [id])
            }
            model B {
                id Int @id
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let table = result.table_bang("A");
    let column = table.column_bang("b_id");
    assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
    assert_eq!(
        table.foreign_keys,
        &[ForeignKey {
            constraint_name: match api.sql_family() {
                SqlFamily::Postgres => Some("A_b_id_fkey".to_owned()),
                SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                SqlFamily::Sqlite => None,
            },
            columns: vec![column.name.clone()],
            referenced_table: "B".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
        }]
    );

    let dm2 = r#"
            model A {
                id Int @id
                b_id String
                b  B   @relation(fields: [b_id], references: [id])
            }
            model B {
                id String @id @default(cuid())
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let table = result.table_bang("A");
    let column = table.column_bang("b_id");
    assert_eq!(column.tpe.family, ColumnTypeFamily::String);
    assert_eq!(
        table.foreign_keys,
        &[ForeignKey {
            constraint_name: match api.sql_family() {
                SqlFamily::Postgres => Some("A_b_id_fkey".to_owned()),
                SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                SqlFamily::Sqlite => None,
            },
            columns: vec!["b_id".into()],
            referenced_table: "B".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
        }]
    );
}

#[test_each_connector]
async fn updating_db_name_of_a_scalar_field_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id String @id @default(cuid())
                field String @map(name:"name1")
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    assert_eq!(result.table_bang("A").column("name1").is_some(), true);

    let dm2 = r#"
            model A {
                id String @id @default(cuid())
                field String @map(name:"name2")
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    assert_eq!(result.table_bang("A").column("name1").is_some(), false);
    assert_eq!(result.table_bang("A").column("name2").is_some(), true);
}

#[test_each_connector]
async fn changing_a_relation_field_to_a_scalar_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            b Int
            b_rel B @relation(fields: [b], references: [id])
        }
        model B {
            id Int @id
            a A // remove this once the implicit back relation field is implemented
        }
    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b", |col| col.assert_type_is_int())?
            .assert_foreign_keys_count(1)?
            .assert_has_fk(&ForeignKey {
                constraint_name: match api.sql_family() {
                    SqlFamily::Postgres => Some("A_b_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec!["b".to_owned()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::Cascade,
            })
    })?;

    let dm2 = r#"
        model A {
            id Int @id
            b String
        }
        model B {
            id Int @id
        }
    "#;

    let result = api.infer_apply(dm2).send().await?.into_inner();

    anyhow::ensure!(result.warnings.is_empty(), "Warnings should be empty");

    let schema = api.assert_schema().await?.into_schema();

    let table = schema.table_bang("A");
    let column = table.column_bang("b");
    assert_eq!(column.tpe.family, ColumnTypeFamily::String);
    assert_eq!(table.foreign_keys, vec![]);

    Ok(())
}

#[test_each_connector]
async fn changing_a_scalar_field_to_a_relation_field_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                b String
            }
            model B {
                id Int @id
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let table = result.table_bang("A");
    let column = table.column_bang("b");
    assert_eq!(column.tpe.family, ColumnTypeFamily::String);
    assert_eq!(table.foreign_keys, vec![]);

    let dm2 = r#"
            model A {
                id Int @id
                b Int
                b_rel B @relation(fields: [b], references: [id])
            }
            model B {
                id Int @id
                a A // remove this once the implicit back relation field is implemented
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let table = result.table_bang("A");
    let column = result.table_bang("A").column_bang("b");
    assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
    assert_eq!(
        table.foreign_keys,
        &[ForeignKey {
            constraint_name: match api.sql_family() {
                SqlFamily::Postgres => Some("A_b_fkey".to_owned()),
                SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                SqlFamily::Sqlite => None,
            },
            columns: vec![column.name.clone()],
            referenced_table: "B".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
        }]
    );
}

#[test_each_connector]
async fn adding_a_many_to_many_relation_must_result_in_a_prisma_style_relation_table(api: &TestApi) -> TestResult {
    let dm1 = r##"
        model A {
            id Int @id
            bs B[]
        }

        model B {
            id String @id
            as A[]
        }
    "##;

    api.infer_apply(&dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("_AToB", |table| {
        table
            .assert_columns_count(2)?
            .assert_column("A", |col| col.assert_type_is_int())?
            .assert_column("B", |col| col.assert_type_is_string())?
            .assert_fk_on_columns(&["A"], |fk| {
                fk.assert_references("A", &["id"])?.assert_cascades_on_delete()
            })?
            .assert_fk_on_columns(&["B"], |fk| {
                fk.assert_references("B", &["id"])?.assert_cascades_on_delete()
            })
    })?;

    Ok(())
}

#[test_each_connector]
async fn adding_a_many_to_many_relation_with_custom_name_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                bs B[] @relation(name: "my_relation")
            }
            model B {
                id Int @id
                as A[] @relation(name: "my_relation")
            }
        "#;

    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let relation_table = result.table_bang("_my_relation");
    assert_eq!(relation_table.columns.len(), 2);

    let a_column = relation_table.column_bang("A");
    assert_eq!(a_column.tpe.family, ColumnTypeFamily::Int);
    let b_column = relation_table.column_bang("B");
    assert_eq!(b_column.tpe.family, ColumnTypeFamily::Int);

    assert_eq!(
        relation_table.foreign_keys,
        vec![
            ForeignKey {
                constraint_name: match api.sql_family() {
                    SqlFamily::Postgres => Some("_my_relation_A_fkey".to_owned()),
                    SqlFamily::Mysql => Some("_my_relation_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![a_column.name.clone()],
                referenced_table: "A".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::Cascade,
            },
            ForeignKey {
                constraint_name: match api.sql_family() {
                    SqlFamily::Postgres => Some("_my_relation_B_fkey".to_owned()),
                    SqlFamily::Mysql => Some("_my_relation_ibfk_2".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![b_column.name.clone()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::Cascade,
            }
        ]
    );
}

#[test_each_connector]
async fn adding_an_inline_relation_must_result_in_a_foreign_key_in_the_model_table(api: &TestApi) -> TestResult {
    let dm1 = r#"
            model A {
                id Int @id
                bid Int
                cid Int?
                b  B   @relation(fields: [bid], references: [id])
                c  C?  @relation(fields: [cid], references: [id])
            }

            model B {
                id Int @id
            }

            model C {
                id Int @id
            }
        "#;

    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let table = result.table_bang("A");

    let b_column = table.column_bang("bid");
    assert_eq!(b_column.tpe.family, ColumnTypeFamily::Int);
    assert_eq!(b_column.tpe.arity, ColumnArity::Required);

    let c_column = table.column_bang("cid");
    assert_eq!(c_column.tpe.family, ColumnTypeFamily::Int);
    assert_eq!(c_column.tpe.arity, ColumnArity::Nullable);

    assert_eq!(
        table.foreign_keys,
        &[
            ForeignKey {
                constraint_name: match api.sql_family() {
                    SqlFamily::Postgres => Some("A_bid_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![b_column.name.clone()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::Cascade, // required relations can't set ON DELETE SET NULL
            },
            ForeignKey {
                constraint_name: match api.sql_family() {
                    SqlFamily::Postgres => Some("A_cid_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_2".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec![c_column.name.clone()],
                referenced_table: "C".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
            }
        ]
    );

    Ok(())
}

#[test_each_connector]
async fn specifying_a_db_name_for_an_inline_relation_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                b_id_field Int @map(name: "b_column")
                b B @relation(fields: [b_id_field], references: [id])
            }

            model B {
                id Int @id
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let table = result.table_bang("A");
    let column = table.column_bang("b_column");
    assert_eq!(column.tpe.family, ColumnTypeFamily::Int);
    assert_eq!(
        table.foreign_keys,
        &[ForeignKey {
            constraint_name: match api.sql_family() {
                SqlFamily::Postgres => Some("A_b_column_fkey".to_owned()),
                SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                SqlFamily::Sqlite => None,
            },
            columns: vec![column.name.clone()],
            referenced_table: "B".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
        }]
    );
}

#[test_each_connector]
async fn adding_an_inline_relation_to_a_model_with_an_exotic_id_type(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                b_id String
                b B @relation(fields: [b_id], references: [id])
            }

            model B {
                id String @id @default(cuid())
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let table = result.table_bang("A");
    let column = table.column_bang("b_id");
    assert_eq!(column.tpe.family, ColumnTypeFamily::String);
    assert_eq!(
        table.foreign_keys,
        &[ForeignKey {
            constraint_name: match api.sql_family() {
                SqlFamily::Postgres => Some("A_b_id_fkey".to_owned()),
                SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                SqlFamily::Sqlite => None,
            },
            columns: vec![column.name.clone()],
            referenced_table: "B".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
        }]
    );
}

#[test_each_connector]
async fn removing_an_inline_relation_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
            model A {
                id Int @id
                b_id Int
                b B @relation(fields: [b_id], references: [id])
            }

            model B {
                id Int @id
            }
        "#;

    api.infer_apply(&dm1).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_table("A", |table| table.assert_has_column("b_id"))?;

    let dm2 = r#"
            model A {
                id Int @id
            }

            model B {
                id Int @id
            }
        "#;

    api.infer_apply(dm2).send().await?.into_inner();

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_foreign_keys_count(0)?
            .assert_indexes_count(0)?
            .assert_does_not_have_column("b")
    })?;

    Ok(())
}

#[test_each_connector]
async fn moving_an_inline_relation_to_the_other_side_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
            model A {
                id Int @id
                b_id Int
                b B @relation(fields: [b_id], references: [id])
            }

            model B {
                id Int @id
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let table = result.table_bang("A");
    assert_eq!(
        table.foreign_keys,
        &[ForeignKey {
            constraint_name: match api.sql_family() {
                SqlFamily::Postgres => Some("A_b_id_fkey".to_owned()),
                SqlFamily::Sqlite => None,
                SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
            },
            columns: vec!["b_id".to_string()],
            referenced_table: "B".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
        }]
    );

    let dm2 = r#"
            model A {
                id Int @id
            }

            model B {
                id Int @id
                a_id Int
                a A @relation(fields: [a_id], references: [id])
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let table = result.table_bang("B");
    assert_eq!(
        table.foreign_keys,
        &[ForeignKey {
            constraint_name: match api.sql_family() {
                SqlFamily::Postgres => Some("B_a_id_fkey".to_owned()),
                SqlFamily::Sqlite => None,
                SqlFamily::Mysql => Some("B_ibfk_1".to_owned()),
            },
            columns: vec!["a_id".to_string()],
            referenced_table: "A".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
        }]
    );

    api.assert_schema()
        .await?
        .assert_table("B", |table| table.assert_foreign_keys_count(1))?
        .assert_table("A", |table| table.assert_foreign_keys_count(0)?.assert_indexes_count(0))
        .map(drop)
}

#[test_each_connector]
async fn adding_a_new_unique_field_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                field String @unique
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result.table_bang("A").indices.iter().find(|i| i.columns == &["field"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);
}

#[test_each_connector]
async fn adding_new_fields_with_multi_column_unique_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField String

                @@unique([field, secondField])
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == vec!["field", "secondField"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);
}

#[test_each_connector]
async fn unique_in_conjunction_with_custom_column_name_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                field String @unique @map("custom_field_name")
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == &["custom_field_name"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);
}

#[test_each_connector]
async fn multi_column_unique_in_conjunction_with_custom_column_name_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                field String @map("custom_field_name")
                secondField String @map("second_custom_field_name")

                @@unique([field, secondField])
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == &["custom_field_name", "second_custom_field_name"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);
}

#[test_each_connector]
async fn removing_an_existing_unique_field_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id    Int    @id
                field String @unique
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == vec!["field"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);

    let dm2 = r#"
            model A {
                id    Int    @id
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == vec!["field"]);
    assert_eq!(index.is_some(), false);
}

#[test_each_connector]
async fn adding_unique_to_an_existing_field_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id    Int    @id
                field String
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == vec!["field"]);
    assert_eq!(index.is_some(), false);

    let dm2 = r#"
            model A {
                id    Int    @id
                field String @unique
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == vec!["field"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);
}

#[test_each_connector]
async fn removing_unique_from_an_existing_field_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id    Int    @id
                field String @unique
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result.table_bang("A").indices.iter().find(|i| i.columns == &["field"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);

    let dm2 = r#"
            model A {
                id    Int    @id
                field String
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let index = result.table_bang("A").indices.iter().find(|i| i.columns == &["field"]);
    assert!(!index.is_some());
}

#[test_each_connector]
async fn removing_multi_field_unique_index_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id    Int    @id
                field String
                secondField Int

                @@unique([field, secondField])
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == &["field", "secondField"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);

    let dm2 = r#"
            model A {
                id    Int    @id
                field String
                secondField Int
            }
        "#;
    let result = api.infer_and_apply(&dm2).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == &["field", "secondField"]);
    assert!(index.is_none());
}

#[test_each_connector]
async fn index_renaming_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customName")
            }
        "#;
    api.infer_apply(&dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table.assert_index_on_columns(&["field", "secondField"], |idx| {
            idx.assert_name("customName")?.assert_is_unique()
        })
    })?;

    let dm2 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customNameA")
            }
        "#;

    let result = api.infer_apply(&dm2).send().await?.into_inner();
    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_name("customNameA"))
    })?;

    // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
    if !api.is_sqlite() {
        let expected_steps = vec![SqlMigrationStep::AlterIndex(AlterIndex {
            table: "A".into(),
            index_new_name: "customNameA".into(),
            index_name: "customName".into(),
        })];
        let actual_steps = result.sql_migration();
        assert_eq!(actual_steps, expected_steps);
    }

    Ok(())
}

#[test_each_connector]
async fn index_renaming_must_work_when_renaming_to_default(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customName")
            }
        "#;
    let result = api.infer_and_apply(&dm1).await;
    let index = result
        .sql_schema
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == &["field", "secondField"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);

    let dm2 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField])
            }
        "#;
    let result = api.infer_and_apply(&dm2).await;
    let indexes = result
        .sql_schema
        .table_bang("A")
        .indices
        .iter()
        .filter(|i| i.columns == &["field", "secondField"] && i.name == "A.field_secondField");
    assert_eq!(indexes.count(), 1);

    // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
    if !api.is_sqlite() {
        let expected_steps = vec![SqlMigrationStep::AlterIndex(AlterIndex {
            table: "A".into(),
            index_new_name: "A.field_secondField".into(),
            index_name: "customName".into(),
        })];
        let actual_steps = result.sql_migration();
        assert_eq!(actual_steps, expected_steps);
    }
}

#[test_each_connector]
async fn index_renaming_must_work_when_renaming_to_custom(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField])
        }
    "#;

    api.infer_apply(&dm1).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    })?;

    let dm2 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "somethingCustom")
        }
    "#;

    let result = api.infer_apply(&dm2).send().await?.assert_green()?.into_inner();
    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["field", "secondField"], |idx| {
                idx.assert_name("somethingCustom")?.assert_is_unique()
            })
    })?;

    // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
    if !api.is_sqlite() {
        let expected_steps = &[SqlMigrationStep::AlterIndex(AlterIndex {
            table: "A".into(),
            index_name: "A.field_secondField".into(),
            index_new_name: "somethingCustom".into(),
        })];
        let actual_steps = result.sql_migration();
        assert_eq!(actual_steps, expected_steps);
    }

    Ok(())
}

#[test_each_connector]
async fn index_updates_with_rename_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customName")
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.name == "customName" && i.columns == &["field", "secondField"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);

    let dm2 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, id], name: "customNameA")
            }
        "#;
    let result = api.infer_and_apply(&dm2).await;
    let indexes = result
        .sql_schema
        .table_bang("A")
        .indices
        .iter()
        .filter(|i| i.columns == &["field", "id"] && i.name == "customNameA");
    assert_eq!(indexes.count(), 1);

    // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
    if !api.is_sqlite() {
        let expected_steps = vec![
            SqlMigrationStep::DropIndex(DropIndex {
                table: "A".into(),
                name: "customName".into(),
            }),
            SqlMigrationStep::CreateIndex(CreateIndex {
                table: "A".into(),
                index: Index {
                    name: "customNameA".into(),
                    columns: vec!["field".into(), "id".into()],
                    tpe: IndexType::Unique,
                },
            }),
        ];
        let actual_steps = result.sql_migration();
        assert_eq!(actual_steps, expected_steps);
    }
}

#[test_each_connector]
async fn dropping_a_model_with_a_multi_field_unique_index_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customName")
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.name == "customName" && i.columns == &["field", "secondField"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);

    let dm2 = "";
    api.infer_apply(&dm2).send().await?.assert_green()?;

    Ok(())
}

#[test_each_connector]
async fn reserved_sql_key_words_must_work(api: &TestApi) {
    // Group is a reserved keyword
    let sql_family = api.sql_family();
    let dm = r#"
            model Group {
                id    String  @default(cuid()) @id
                parent_id String?
                parent Group? @relation(name: "ChildGroups", fields: [parent_id], references: id)
                childGroups Group[] @relation(name: "ChildGroups")
            }
        "#;
    let result = api.infer_and_apply(&dm).await.sql_schema;

    let table = result.table_bang("Group");
    assert_eq!(
        table.foreign_keys,
        vec![ForeignKey {
            constraint_name: match sql_family {
                SqlFamily::Postgres => Some("Group_parent_id_fkey".to_owned()),
                SqlFamily::Mysql => Some("Group_ibfk_1".to_owned()),
                SqlFamily::Sqlite => None,
            },
            columns: vec!["parent_id".to_string()],
            referenced_table: "Group".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::SetNull,
        }]
    );
}

#[test_each_connector]
async fn migrations_with_many_to_many_related_models_must_not_recreate_indexes(api: &TestApi) {
    // test case for https://github.com/prisma/lift/issues/148
    let dm_1 = r#"
            model User {
                id        String  @default(cuid()) @id
            }

            model Profile {
                id        String  @default(cuid()) @id
                userId    String
                user      User    @relation(fields: userId, references: id)
                skills    Skill[]
            }

            model Skill {
                id          String  @default(cuid()) @id
                profiles    Profile[]
            }
        "#;
    let sql_schema = api.infer_and_apply(&dm_1).await.sql_schema;

    let index = sql_schema
        .table_bang("_ProfileToSkill")
        .indices
        .iter()
        .find(|index| index.name == "_ProfileToSkill_AB_unique")
        .expect("index is present");
    assert_eq!(index.tpe, IndexType::Unique);

    let dm_2 = r#"
            model User {
                id        String  @default(cuid()) @id
                someField String?
            }

            model Profile {
                id        String  @default(cuid()) @id
                userId    String
                user      User    @relation(fields: userId, references: id)
                skills    Skill[]
            }

            model Skill {
                id          String  @default(cuid()) @id
                profiles    Profile[]
            }
        "#;

    let result = api.infer_and_apply(&dm_2).await;
    let sql_schema = result.sql_schema;

    let index = sql_schema
        .table_bang("_ProfileToSkill")
        .indices
        .iter()
        .find(|index| index.name == "_ProfileToSkill_AB_unique")
        .expect("index is present");
    assert_eq!(index.tpe, IndexType::Unique);
}

#[test_each_connector]
async fn removing_a_relation_field_must_work(api: &TestApi) -> TestResult {
    let dm_1 = r#"
            model User {
                id        String  @default(cuid()) @id
                address_id String @map("address_name")
                address   Address @relation(fields: [address_id], references: [id])
            }

            model Address {
                id        String  @default(cuid()) @id
                street    String
            }
        "#;

    api.infer_apply(&dm_1).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_table("User", |table| table.assert_has_column("address_name"))?;

    let dm_2 = r#"
            model User {
                id        String  @default(cuid()) @id
            }

            model Address {
                id        String  @default(cuid()) @id
                street    String
            }
        "#;

    let sql_schema = api.infer_and_apply(&dm_2).await.sql_schema;

    let address_name_field = sql_schema
        .table_bang("User")
        .columns
        .iter()
        .find(|col| col.name == "address_name");

    assert!(address_name_field.is_none());

    Ok(())
}

#[test_each_connector]
async fn simple_type_aliases_in_migrations_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        type CUID = String @id @default(cuid())

        model User {
            id CUID
            age Float
        }
    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;

    Ok(())
}

#[test_each_connector]
async fn model_with_multiple_indexes_works(api: &TestApi) -> TestResult {
    let dm = r#"
    model User {
      id         Int       @id
    }

    model Post {
      id        Int       @id
    }

    model Comment {
      id        Int       @id
    }

    model Like {
      id        Int       @id
      user_id   Int
      user      User @relation(fields: [user_id], references: [id])
      post_id   Int
      post      Post @relation(fields: [post_id], references: [id])
      comment_id Int
      comment   Comment @relation(fields: [comment_id], references: [id])

      @@index([post_id])
      @@index([user_id])
      @@index([comment_id])
    }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_table("Like", |table| table.assert_indexes_count(3))?;

    Ok(())
}

#[test_each_connector]
async fn foreign_keys_of_inline_one_to_one_relations_have_a_unique_constraint(api: &TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            box Box
        }

        model Box {
            id Int @id
            cat_id Int
            cat Cat @relation(fields: [cat_id], references: [id])
        }
    "#;

    let schema = api.infer_and_apply(dm).await.sql_schema;

    let box_table = schema.table_bang("Box");

    let expected_indexes = &[Index {
        name: "Box_cat_id".into(),
        columns: vec!["cat_id".into()],
        tpe: IndexType::Unique,
    }];

    assert_eq!(box_table.indices, expected_indexes);
}

#[test_each_connector]
async fn column_defaults_must_be_migrated(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Fruit {
            id Int @id
            name String @default("banana")
        }
    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Fruit", |table| {
        table.assert_column("name", |col| {
            col.assert_default(Some(DefaultValue::VALUE(PrismaValue::String("banana".to_string()))))
        })
    })?;

    let dm2 = r#"
        model Fruit {
            id Int @id
            name String @default("mango")
        }
    "#;

    api.infer_apply(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Fruit", |table| {
        table.assert_column("name", |col| {
            col.assert_default(Some(DefaultValue::VALUE(PrismaValue::String("mango".to_string()))))
        })
    })?;

    Ok(())
}

#[test_each_connector]
async fn escaped_string_defaults_are_not_arbitrarily_migrated(api: &TestApi) -> TestResult {
    use quaint::ast::*;

    let dm1 = r#"
        model Fruit {
            id String @id @default(cuid())
            name String @default("ba\0nana")
            seasonality String @default("\"summer\"")
            contains String @default("'potassium'")
            sideNames String @default("top\ndown")
            size Float @default(12.3)
        }
    "#;

    let output = api.infer_apply(dm1).send().await?.into_inner();

    anyhow::ensure!(!output.datamodel_steps.is_empty(), "Yes migration");
    anyhow::ensure!(output.warnings.is_empty(), "No warnings");

    let insert = Insert::single_into(api.render_table_name("Fruit"))
        .value("id", "apple-id")
        .value("name", "apple")
        .value("sideNames", "stem and the other one")
        .value("contains", "'vitamin C'")
        .value("seasonality", "september");

    api.database().query(insert.into()).await?;

    let output = api.infer_apply(dm1).send().await?.assert_green()?.into_inner();

    anyhow::ensure!(output.datamodel_steps.is_empty(), "No migration");

    let sql_schema = api.describe_database().await?;
    let table = sql_schema.table_bang("Fruit");

    assert_eq!(
        table.column("name").and_then(|c| c.default.clone()),
        Some(if api.is_mysql() && !api.connector_name().contains("mariadb") {
            DefaultValue::VALUE(PrismaValue::String("ba\u{0}nana".to_string()))
        } else {
            DefaultValue::VALUE(PrismaValue::String("ba\\0nana".to_string()))
        })
    );
    assert_eq!(
        table.column("sideNames").and_then(|c| c.default.clone()),
        Some(if api.is_mysql() && !api.connector_name().contains("mariadb") {
            DefaultValue::VALUE(PrismaValue::String("top\ndown".to_string()))
        } else {
            DefaultValue::VALUE(PrismaValue::String("top\\ndown".to_string()))
        })
    );
    assert_eq!(
        table.column("contains").and_then(|c| c.default.clone()),
        Some(DefaultValue::VALUE(PrismaValue::String("potassium".to_string())))
    );
    assert_eq!(
        table.column("seasonality").and_then(|c| c.default.clone()),
        Some(DefaultValue::VALUE(PrismaValue::String("summer".to_string())))
    );

    Ok(())
}

#[test_each_connector]
async fn created_at_does_not_get_arbitrarily_migrated(api: &TestApi) -> TestResult {
    use quaint::ast::*;

    let dm1 = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String
            createdAt DateTime @default(now())
        }
    "#;

    let schema = api.infer_and_apply(dm1).await.sql_schema;

    let insert = Insert::single_into(api.render_table_name("Fruit")).value("name", "banana");
    api.database().query(insert.into()).await.unwrap();

    anyhow::ensure!(
        matches!(
            schema.table_bang("Fruit").column_bang("createdAt").default,
            Some(DefaultValue::NOW)
        ),
        "createdAt default is set"
    );

    let dm2 = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String
            createdAt DateTime @default(now())
        }
    "#;

    let output = api.infer_apply(dm2).send().await?.assert_green()?.into_inner();

    anyhow::ensure!(output.warnings.is_empty(), "No warnings");
    anyhow::ensure!(output.datamodel_steps.is_empty(), "Migration should be empty");

    Ok(())
}

#[test_each_connector(tags("sqlite"))]
async fn renaming_a_datasource_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        datasource db1 {
            provider = "sqlite"
            url = "file:///tmp/prisma-test.db"
        }

        model User {
            id Int @id
        }
    "#;

    let infer_output = api.infer(dm1.to_owned()).send().await?;

    let dm2 = r#"
        datasource db2 {
            provider = "sqlite"
            url = "file:///tmp/prisma-test.db"
        }

        model User {
            id Int @id
        }
    "#;

    api.infer(dm2.to_owned())
        .assume_to_be_applied(Some(infer_output.datamodel_steps))
        .migration_id(Some("mig02".to_owned()))
        .send()
        .await?;

    Ok(())
}

#[test_each_connector]
async fn relations_can_reference_arbitrary_unique_fields(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            id Int @id
            email String @unique
        }

        model Account {
            id Int @id
            uem String
            user User @relation(fields: [uem], references: [email])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    let schema = api.describe_database().await?;

    let fks = &schema.table_bang("Account").foreign_keys;

    assert_eq!(fks.len(), 1);

    let fk = fks.iter().next().unwrap();

    assert_eq!(fk.columns, &["uem"]);
    assert_eq!(fk.referenced_table, "User");
    assert_eq!(fk.referenced_columns, &["email"]);

    Ok(())
}

#[test_each_connector]
async fn relations_can_reference_arbitrary_unique_fields_with_maps(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            id Int @id
            email String @unique @map("emergency-mail")
            accounts Account[]

            @@map("users")
        }

        model Account {
            id Int @id
            uem String @map("user-id")
            user User @relation(fields: [uem], references: [email])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["user-id"], |fk| fk.assert_references("users", &["emergency-mail"]))
    })?;

    Ok(())
}

#[test_each_connector]
async fn relations_can_reference_multiple_fields(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            id Int @id
            email  String
            age    Int

            @@unique([email, age])
        }

        model Account {
            id   Int @id
            usermail String
            userage Int
            user User @relation(fields: [usermail, userage], references: [email, age])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["usermail", "userage"], |fk| {
                fk.assert_references("User", &["email", "age"])
            })
    })?;

    Ok(())
}

#[test_each_connector]
async fn relations_with_mappings_on_both_sides_can_reference_multiple_fields(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            id Int @id
            email  String @map("emergency-mail")
            age    Int    @map("birthdays-count")

            @@unique([email, age])
            @@map("users")
        }

        model Account {
            id   Int @id
            usermail String @map("emergency-mail-fk-1")
            userage Int @map("age-fk2")

            user User @relation(fields: [usermail, userage], references: [email, age])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["emergency-mail-fk-1", "age-fk2"], |fk| {
                fk.assert_references("users", &["emergency-mail", "birthdays-count"])
            })
    })?;

    Ok(())
}

#[test_each_connector]
async fn relations_with_mappings_on_referenced_side_can_reference_multiple_fields(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            id Int @id
            email  String @map("emergency-mail")
            age    Int    @map("birthdays-count")

            @@unique([email, age])
            @@map("users")
        }

        model Account {
            id   Int @id
            useremail String
            userage Int
            user User @relation(fields: [useremail, userage], references: [email, age])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["useremail", "userage"], |fk| {
                fk.assert_references("users", &["emergency-mail", "birthdays-count"])
            })
    })?;

    Ok(())
}

#[test_each_connector]
async fn relations_with_mappings_on_referencing_side_can_reference_multiple_fields(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            id Int @id
            email  String
            age    Int

            @@unique([email, age])
            @@map("users")
        }

        model Account {
            id   Int @id
            user_email String @map("emergency-mail-fk1")
            user_age Int @map("age-fk2")
            user User @relation(fields: [user_email, user_age], references: [email, age])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["emergency-mail-fk1", "age-fk2"], |fk| {
                fk.assert_references("users", &["email", "age"])
            })
    })?;

    Ok(())
}

#[test_each_connector]
async fn foreign_keys_are_added_on_existing_tables(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model User {
            id Int @id
            email String @unique
        }

        model Account {
            id Int @id
        }
    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        // There should be no foreign keys yet.
        .assert_table("Account", |table| table.assert_foreign_keys_count(0))?;

    let dm2 = r#"
        model User {
            id Int @id
            email String @unique
        }

        model Account {
            id Int @id
            user_email String
            user User @relation(fields: [user_email], references: [email])
        }
    "#;

    api.infer_apply(dm2).send().await?.assert_green()?;
    api.assert_schema().await?.assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
    })?;

    Ok(())
}

#[test_each_connector]
async fn basic_compound_primary_keys_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            firstName String
            lastName String

            @@id([lastName, firstName])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("User", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["lastName", "firstName"]))
    })?;

    Ok(())
}

#[test_each_connector]
async fn compound_primary_keys_on_mapped_columns_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            firstName String @map("first_name")
            lastName String @map("family_name")

            @@id([firstName, lastName])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("User", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["first_name", "family_name"]))
    })?;

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn references_to_models_with_compound_primary_keys_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            firstName String
            lastName  String
            pets      Pet[]

            @@id([firstName, lastName])
        }

        model Pet {
            id              String @id
            human_firstName String
            human_lastName  String

            human User @relation(fields: [human_firstName, human_lastName], references: [firstName, lastName])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    let sql_schema = api.describe_database().await?;

    sql_schema
        .assert_table("Pet")?
        .assert_has_column("id")?
        .assert_has_column("human_firstName")?
        .assert_has_column("human_lastName")?
        .assert_foreign_keys_count(1)?
        .assert_fk_on_columns(&["human_firstName", "human_lastName"], |fk| {
            fk.assert_references("User", &["firstName", "lastName"])
        })?;

    Ok(())
}

#[test_each_connector]
async fn join_tables_between_models_with_compound_primary_keys_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Human {
            firstName String
            lastName String
            cats Cat[]

            @@id([firstName, lastName])
        }

        model Cat {
            id String @id
            humans Human[]
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("_CatToHuman", |table| {
        table
            .assert_has_column("B_firstName")?
            .assert_has_column("B_lastName")?
            .assert_has_column("A")?
            .assert_fk_on_columns(&["B_firstName", "B_lastName"], |fk| {
                fk.assert_references("Human", &["firstName", "lastName"])?
                    .assert_cascades_on_delete()
            })?
            .assert_fk_on_columns(&["A"], |fk| {
                fk.assert_references("Cat", &["id"])?.assert_cascades_on_delete()
            })?
            .assert_indexes_count(2)?
            .assert_index_on_columns(&["A", "B_firstName", "B_lastName"], |idx| idx.assert_is_unique())?
            .assert_index_on_columns(&["B_firstName", "B_lastName"], |idx| idx.assert_is_not_unique())
    })?;

    Ok(())
}

#[test_each_connector]
async fn join_tables_between_models_with_mapped_compound_primary_keys_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Human {
            firstName String @map("the_first_name")
            lastName String @map("the_last_name")
            cats Cat[]

            @@id([firstName, lastName])
        }

        model Cat {
            id String @id
            humans Human[]
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    let sql_schema = api.describe_database().await?;

    sql_schema
        .assert_table("_CatToHuman")?
        .assert_has_column("B_the_first_name")?
        .assert_has_column("B_the_last_name")?
        .assert_has_column("A")?
        .assert_fk_on_columns(&["B_the_first_name", "B_the_last_name"], |fk| {
            fk.assert_references("Human", &["the_first_name", "the_last_name"])
        })?
        .assert_fk_on_columns(&["A"], |fk| fk.assert_references("Cat", &["id"]))?
        .assert_indexes_count(2)?;

    Ok(())
}

#[test_each_connector]
async fn switching_databases_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        datasource db {
            provider = "sqlite"
            url = "file:dev.db"
        }

        model Test {
            id String @id
            name String
        }
    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;

    // Drop the existing migrations.
    api.migration_persistence().reset().await?;

    let dm2 = r#"
        datasource db {
            provider = "sqlite"
            url = "file:hiya.db"
        }

        model Test {
            id String @id
            name String
        }
    "#;

    api.infer_apply(dm2).send().await?.assert_green()?;

    Ok(())
}

#[test_each_connector]
async fn adding_mutual_references_on_existing_tables_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
        }

        model B {
            id Int @id
        }
    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;

    let dm2 = r#"
        model A {
            id Int
            name String @unique
            b_email String
            brel B @relation("AtoB", fields: [b_email], references: [email])
        }

        model B {
            id Int
            email String @unique
            a_name String
            arel A @relation("BtoA", fields: [a_name], references: [name])
        }
    "#;

    api.infer_apply(dm2).send().await?.assert_green()?;

    Ok(())
}
