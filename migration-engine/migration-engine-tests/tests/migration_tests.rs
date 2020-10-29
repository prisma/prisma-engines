mod apply_migration;
mod apply_migrations;
mod calculate_database_steps;
mod create_migration;
mod datamodel_calculator;
mod datamodel_steps_inferrer;
mod diagnose_migration_history;
mod errors;
mod evaluate_data_loss;
mod existing_data;
mod existing_databases;
mod infer_migration_steps;
mod initialization;
mod migration_persistence;
mod migrations;
mod reset;
mod schema_push;
mod unapply_migration;

use migration_engine_tests::sql::*;
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use quaint::prelude::{Queryable, SqlFamily};
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
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table
            .assert_columns_count(6)?
            .assert_column("int", |c| {
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::Int)
            })?
            .assert_column("float", |c| {
                //The native types work made the inferrence more correct on the describer level.
                // But unless the feature is activated, this will be mapped to float like before in the datamodel level
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::Decimal)
            })?
            .assert_column("boolean", |c| {
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::Boolean)
            })?
            .assert_column("string", |c| {
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::String)
            })?
            .assert_column("dateTime", |c| {
                c.assert_is_required()?.assert_type_family(ColumnTypeFamily::DateTime)
            })
    })?;

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn adding_an_enum_field_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
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
            .assert_columns_count(2)?
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

    api.schema_push(dm2).send().await?.assert_green()?;
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
async fn adding_an_id_field_of_type_int_with_autoincrement_works(api: &TestApi) {
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

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
#[test_each_connector(ignore("sqlite"))]
async fn making_an_existing_id_field_autoincrement_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Post {
            id        Int        @id
            content   String?
            createdAt DateTime
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime
        }
    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_no_autoincrement())
    })?;

    let dm2 = r#"
        model Post {
            id        Int        @id @default(autoincrement())
            content   String?
            createdAt DateTime
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime
        }
    "#;

    api.infer_apply(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_autoincrement())
    })?;

    // Check that the migration is idempotent.
    api.infer_apply(dm2).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
#[test_each_connector(ignore("sqlite"))]
async fn removing_autoincrement_from_an_existing_field_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Post {
            id        Int        @id @default(autoincrement())
            content   String?
            createdAt DateTime
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime
        }
    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_autoincrement())
    })?;

    let dm2 = r#"
        model Post {
            id        Int        @id
            content   String?
            createdAt DateTime
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime
        }
    "#;

    api.infer_apply(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_no_autoincrement())
    })?;

    // Check that the migration is idempotent.
    api.infer_apply(dm2)
        .migration_id(Some("idempotency-check"))
        .send()
        .await?
        .assert_green()?
        .assert_no_steps()?;

    Ok(())
}

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
#[test_each_connector(ignore("sqlite"))]
async fn flipping_autoincrement_on_and_off_works(api: &TestApi) -> TestResult {
    let dm_without = r#"
        model Post {
            id        Int        @id
            title     String     @default("")
        }
    "#;

    let dm_with = r#"
        model Post {
            id        Int        @id @default(autoincrement())
            updatedAt DateTime
        }
    "#;

    api.infer_apply(dm_with).send().await?.assert_green()?;
    api.infer_apply(dm_without).send().await?.assert_green()?;
    api.infer_apply(dm_with).send().await?.assert_green()?;
    api.infer_apply(dm_without).send().await?.assert_green()?;
    api.infer_apply(dm_with).send().await?.assert_green()?;

    Ok(())
}

// Ignoring sqlite is OK, because sqlite integer primary keys are always auto-incrementing.
#[test_each_connector(ignore("sqlite"))]
async fn making_an_autoincrement_default_an_expression_then_autoincrement_again_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Post {
            id        Int        @id @default(autoincrement())
            title     String     @default("")
        }
    "#;

    api.infer_apply(dm1)
        .migration_id(Some("apply_dm1"))
        .send()
        .await?
        .assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_autoincrement())
    })?;

    let dm2 = r#"
        model Post {
            id        Int       @id @default(3)
            title     String    @default("")
        }
    "#;

    api.infer_apply(dm2)
        .migration_id(Some("apply_dm2"))
        .send()
        .await?
        .assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model
            .assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_no_autoincrement())?
            .assert_column("id", |column| {
                column.assert_default(Some(DefaultValue::VALUE(PrismaValue::Int(3))))
            })
    })?;

    // Now re-apply the sequence.
    api.infer_apply(dm1)
        .migration_id(Some("apply_dm1_again"))
        .send()
        .await?
        .assert_green()?;

    api.assert_schema().await?.assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"])?.assert_has_autoincrement())
    })?;

    Ok(())
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
async fn update_type_of_scalar_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id @default(cuid())
            field String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table.assert_column("field", |column| column.assert_type_is_string())
    })?;

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            field Int
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table.assert_column("field", |column| column.assert_type_is_int())
    })?;

    Ok(())
}

#[test_each_connector]
async fn changing_the_type_of_an_id_field_must_work(api: &TestApi) -> TestResult {
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

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_family(ColumnTypeFamily::Int))?
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["id"]))
    })?;

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

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_family(ColumnTypeFamily::String))?
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["id"]))
    })?;

    Ok(())
}

#[test_each_connector]
async fn changing_the_type_of_a_field_referenced_by_a_fk_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            b_id Int
            b  B   @relation(fields: [b_id], references: [uniq])
        }

        model B {
            uniq Int @unique
            name String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_family(ColumnTypeFamily::Int))?
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["uniq"]))
    })?;

    let dm2 = r#"
        model A {
            id Int @id
            b_id String
            b  B   @relation(fields: [b_id], references: [uniq])
        }

        model B {
            uniq String @unique @default(cuid())
            name String
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b_id", |col| col.assert_type_family(ColumnTypeFamily::String))?
            .assert_fk_on_columns(&["b_id"], |fk| fk.assert_references("B", &["uniq"]))
    })?;

    Ok(())
}

#[test_each_connector]
async fn updating_db_name_of_a_scalar_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id String @id @default(cuid())
            field String @map(name:"name1")
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_table("A", |table| table.assert_has_column("name1"))?;

    let dm2 = r#"
        model A {
            id String @id @default(cuid())
            field String @map(name:"name2")
        }
    "#;

    let result = api.infer_and_apply(&dm2).await.sql_schema;
    assert_eq!(result.table_bang("A").column("name1").is_some(), false);
    assert_eq!(result.table_bang("A").column("name2").is_some(), true);

    Ok(())
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

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b", |col| col.assert_type_is_int())?
            .assert_foreign_keys_count(1)?
            .assert_has_fk(&ForeignKey {
                constraint_name: match api.sql_family() {
                    SqlFamily::Postgres => Some("A_b_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                    SqlFamily::Mssql => Some("A_b_fkey".to_owned()),
                },
                columns: vec!["b".to_owned()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::Cascade,
                on_update_action: ForeignKeyAction::NoAction,
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

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("b", |col| col.assert_type_is_string())?
            .assert_foreign_keys_count(0)
    })?;

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
            a A
        }
    "#;
    let result = api.infer_and_apply_forcefully(&dm2).await.sql_schema;
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
                SqlFamily::Mssql => Some("A_b_fkey".to_owned()),
            },
            columns: vec![column.name.clone()],
            referenced_table: "B".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
            on_update_action: ForeignKeyAction::NoAction,
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

    api.schema_push(dm1).send().await?.assert_green()?;

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
async fn adding_a_many_to_many_relation_with_custom_name_must_work(api: &TestApi) -> TestResult {
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

    api.infer_apply(&dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("_my_relation", |table| {
        table
            .assert_columns_count(2)?
            .assert_column("A", |col| col.assert_type_is_int())?
            .assert_column("B", |col| col.assert_type_is_int())?
            .assert_foreign_keys_count(2)?
            .assert_fk_on_columns(&["A"], |fk| fk.assert_references("A", &["id"]))?
            .assert_fk_on_columns(&["B"], |fk| fk.assert_references("B", &["id"]))
    })?;

    Ok(())
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
                    SqlFamily::Mssql => Some("A_bid_fkey".to_owned()),
                },
                columns: vec![b_column.name.clone()],
                referenced_table: "B".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::Cascade, // required relations can't set ON DELETE SET NULL
                on_update_action: ForeignKeyAction::NoAction,
            },
            ForeignKey {
                constraint_name: match api.sql_family() {
                    SqlFamily::Postgres => Some("A_cid_fkey".to_owned()),
                    SqlFamily::Mysql => Some("A_ibfk_2".to_owned()),
                    SqlFamily::Sqlite => None,
                    SqlFamily::Mssql => Some("A_cid_fkey".to_owned()),
                },
                columns: vec![c_column.name.clone()],
                referenced_table: "C".to_string(),
                referenced_columns: vec!["id".to_string()],
                on_delete_action: ForeignKeyAction::SetNull,
                on_update_action: ForeignKeyAction::NoAction,
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
                SqlFamily::Mssql => Some("A_b_column_fkey".to_owned()),
            },
            columns: vec![column.name.clone()],
            referenced_table: "B".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
            on_update_action: ForeignKeyAction::NoAction,
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
                SqlFamily::Mssql => Some("A_b_id_fkey".to_owned()),
            },
            columns: vec![column.name.clone()],
            referenced_table: "B".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
            on_update_action: ForeignKeyAction::NoAction,
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
                SqlFamily::Mssql => Some("A_b_id_fkey".to_owned()),
            },
            columns: vec!["b_id".to_string()],
            referenced_table: "B".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
            on_update_action: ForeignKeyAction::NoAction,
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
                SqlFamily::Mssql => Some("B_a_id_fkey".to_owned()),
            },
            columns: vec!["a_id".to_string()],
            referenced_table: "A".to_string(),
            referenced_columns: vec!["id".to_string()],
            on_delete_action: ForeignKeyAction::Cascade,
            on_update_action: ForeignKeyAction::NoAction,
        }]
    );

    api.assert_schema()
        .await?
        .assert_table("B", |table| table.assert_foreign_keys_count(1))?
        .assert_table("A", |table| table.assert_foreign_keys_count(0)?.assert_indexes_count(0))?;

    Ok(())
}

#[test_each_connector]
async fn adding_a_new_unique_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            field String @unique
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |index| index.assert_is_unique())
    })?;

    Ok(())
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
async fn adding_unique_to_an_existing_field_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id    Int    @id
            field String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("A", |table| table.assert_indexes_count(0))?;

    let dm2 = r#"
        model A {
            id    Int    @id
            field String @unique
        }
    "#;

    api.schema_push(dm2)
        .force(true)
        .send()
        .await?
        .assert_executable()?
        .assert_warnings(&["The migration will add a unique constraint covering the columns `[field]` on the table `A`. If there are existing duplicate values, the migration will fail.".into()])?
        .assert_has_executed_steps()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    })?;

    Ok(())
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
    assert!(index.is_none());
}

#[test_each_connector]
async fn reserved_sql_key_words_must_work(api: &TestApi) -> TestResult {
    // Group is a reserved keyword
    let dm = r#"
        model Group {
            id          String  @id @default(cuid())
            parent_id   String?
            parent      Group? @relation(name: "ChildGroups", fields: [parent_id], references: id)
            childGroups Group[] @relation(name: "ChildGroups")
        }
    "#;

    api.infer_apply(&dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Group", |table| {
        table.assert_fk_on_columns(&["parent_id"], |fk| fk.assert_references("Group", &["id"]))
    })?;

    Ok(())
}

#[test_each_connector]
async fn migrations_with_many_to_many_related_models_must_not_recreate_indexes(api: &TestApi) {
    // test case for https://github.com/prisma/lift/issues/148
    let dm_1 = r#"
        model User {
            id        String  @id @default(cuid())
        }

        model Profile {
            id        String  @id @default(cuid())
            userId    String
            user      User    @relation(fields: userId, references: id)
            skills    Skill[]
        }

        model Skill {
            id          String  @id @default(cuid())
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
            id        String  @id @default(cuid())
            someField String?
        }

        model Profile {
            id        String  @id @default(cuid())
            userId    String
            user      User    @relation(fields: userId, references: id)
            skills    Skill[]
        }

        model Skill {
            id          String  @id @default(cuid())
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
            id        String  @id @default(cuid())
            address_id String @map("address_name")
            address   Address @relation(fields: [address_id], references: [id])
        }

        model Address {
            id        String  @id @default(cuid())
            street    String
        }
    "#;

    api.infer_apply(&dm_1).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_table("User", |table| table.assert_has_column("address_name"))?;

    let dm_2 = r#"
        model User {
            id        String  @id @default(cuid())
        }

        model Address {
            id        String  @id @default(cuid())
            street    String
        }
    "#;

    api.infer_apply(dm_2).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("User", |table| table.assert_does_not_have_column("address_name"))?;

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
        name: "Box_cat_id_unique".into(),
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
    use quaint::ast::Insert;

    let dm1 = r#"
        model Fruit {
            id String @id @default(cuid())
            seasonality String @default("\"summer\"")
            contains String @default("'potassium'")
            sideNames String @default("top\ndown")
            size Float @default(12.3)
        }
    "#;

    api.infer_apply(dm1)
        .migration_id(Some("first migration"))
        .send()
        .await?
        .assert_green()?
        .into_inner();

    let insert = Insert::single_into(api.render_table_name("Fruit"))
        .value("id", "apple-id")
        .value("sideNames", "stem and the other one")
        .value("contains", "'vitamin C'")
        .value("seasonality", "september");

    api.database().query(insert.into()).await?;

    api.infer_apply(dm1)
        .migration_id(Some("second migration"))
        .send()
        .await?
        .assert_green()?
        .assert_no_steps()?;

    let sql_schema = api.describe_database().await?;
    let table = sql_schema.table_bang("Fruit");

    assert_eq!(
        table.column("sideNames").and_then(|c| c.default.clone()),
        Some(DefaultValue::VALUE(PrismaValue::String("top\ndown".to_string())))
    );
    assert_eq!(
        table.column("contains").and_then(|c| c.default.clone()),
        Some(DefaultValue::VALUE(PrismaValue::String("'potassium'".to_string())))
    );
    assert_eq!(
        table.column("seasonality").and_then(|c| c.default.clone()),
        Some(DefaultValue::VALUE(PrismaValue::String(r#""summer""#.to_string())))
    );

    Ok(())
}

#[test_each_connector]
async fn created_at_does_not_get_arbitrarily_migrated(api: &TestApi) -> TestResult {
    use quaint::ast::Insert;

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

    api.schema_push(dm2).send().await?.assert_green()?.assert_no_steps()?;

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

    api.schema_push(dm).send().await?.assert_green()?;

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

    api.schema_push(dm).send().await?.assert_green()?;

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
async fn a_relation_with_mappings_on_both_sides_can_reference_multiple_fields(api: &TestApi) -> TestResult {
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

    api.schema_push(dm).send().await?.assert_green()?;

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

    api.schema_push(dm).send().await?.assert_green()?;

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

    api.schema_push(dm1).send().await?.assert_green()?;

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

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
    })?;

    Ok(())
}

#[test_each_connector(log = "debug,sql_schema_describer=info")]
async fn foreign_keys_can_be_added_on_existing_columns(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model User {
            id Int @id
            email String @unique
        }

        model Account {
            id Int @id
            user_email String
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

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

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
    })?;

    Ok(())
}

#[test_each_connector]
async fn foreign_keys_can_be_dropped_on_existing_columns(api: &TestApi) -> TestResult {
    let dm1 = r#"
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

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Account", |table| {
        table
            .assert_foreign_keys_count(1)?
            .assert_fk_on_columns(&["user_email"], |fk| fk.assert_references("User", &["email"]))
    })?;

    let dm2 = r#"
        model User {
            id Int @id
            email String @unique
        }

        model Account {
            id Int @id
            user_email String
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Account", |table| table.assert_foreign_keys_count(0))?;

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

    api.schema_push(dm).send().await?.assert_green()?;

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

    api.schema_push(dm).send().await?.assert_green()?;

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

    api.schema_push(dm).send().await?.assert_green()?;

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
            cats HumanToCat[]

            @@id([firstName, lastName])
        }

        model HumanToCat {
            human_firstName String
            human_lastName String
            cat_id String

            cat Cat @relation(fields: [cat_id], references: [id])
            human Human @relation(fields: [human_firstName, human_lastName], references: [firstName, lastName])

            @@unique([cat_id, human_firstName, human_lastName], name: "joinTableUnique")
            @@index([human_firstName, human_lastName], name: "joinTableIndex")
        }

        model Cat {
            id String @id
            humans HumanToCat[]
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("HumanToCat", |table| {
        table
            .assert_has_column("human_firstName")?
            .assert_has_column("human_lastName")?
            .assert_has_column("cat_id")?
            .assert_fk_on_columns(&["human_firstName", "human_lastName"], |fk| {
                fk.assert_references("Human", &["firstName", "lastName"])?
                    .assert_cascades_on_delete()
            })?
            .assert_fk_on_columns(&["cat_id"], |fk| {
                fk.assert_references("Cat", &["id"])?.assert_cascades_on_delete()
            })?
            .assert_indexes_count(2)?
            .assert_index_on_columns(&["cat_id", "human_firstName", "human_lastName"], |idx| {
                idx.assert_is_unique()
            })?
            .assert_index_on_columns(&["human_firstName", "human_lastName"], |idx| idx.assert_is_not_unique())
    })?;

    Ok(())
}

#[test_each_connector]
async fn join_tables_between_models_with_mapped_compound_primary_keys_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Human {
            firstName String @map("the_first_name")
            lastName String @map("the_last_name")
            cats HumanToCat[]

            @@id([firstName, lastName])
        }

        model HumanToCat {
            human_the_first_name String
            human_the_last_name String
            cat_id String

            cat Cat @relation(fields: [cat_id], references: [id])
            human Human @relation(fields: [human_the_first_name, human_the_last_name], references: [firstName, lastName])

            @@unique([human_the_first_name, human_the_last_name, cat_id], name: "joinTableUnique")
            @@index([cat_id])
        }

        model Cat {
            id String @id
            humans HumanToCat[]
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    let sql_schema = api.describe_database().await?;

    sql_schema
        .assert_table("HumanToCat")?
        .assert_has_column("human_the_first_name")?
        .assert_has_column("human_the_last_name")?
        .assert_has_column("cat_id")?
        .assert_fk_on_columns(&["human_the_first_name", "human_the_last_name"], |fk| {
            fk.assert_references("Human", &["the_first_name", "the_last_name"])
        })?
        .assert_fk_on_columns(&["cat_id"], |fk| fk.assert_references("Cat", &["id"]))?
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

    api.schema_push(dm1).send().await?.assert_green()?;

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

    api.schema_push(dm2).send().await?.assert_green()?;

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

    api.schema_push(dm1).send().await?.assert_green()?;

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

    let res = api.schema_push(dm2).force(true).send().await?;

    if api.sql_family().is_sqlite() {
        res.assert_green()?;
    } else {
        res.assert_warnings(&["The migration will add a unique constraint covering the columns `[name]` on the table `A`. If there are existing duplicate values, the migration will fail.".into(), "The migration will add a unique constraint covering the columns `[email]` on the table `B`. If there are existing duplicate values, the migration will fail.".into()])?;
    };

    Ok(())
}

#[test_each_connector]
async fn schemas_with_dbgenerated_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
    model User {
        age         Int?
        createdAt   DateTime  @default(dbgenerated())
        email       String?
        firstName   String    @default("")
        id          Int       @id @default(autoincrement())
        lastName    String    @default("")
        password    String?
        updatedAt   DateTime  @default(dbgenerated())
    }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    Ok(())
}

#[test_each_connector]
async fn models_with_an_autoincrement_field_as_part_of_a_multi_field_id_can_be_created(api: &TestApi) -> TestResult {
    let dm = r#"
        model List {
            id        Int  @id @default(autoincrement())
            uList     String? @unique
            todoId    Int @default(1)
            todoName  String
            todo      Todo   @relation(fields: [todoId, todoName], references: [id, uTodo])
        }

        model Todo {
            id     Int @default(autoincrement())
            uTodo  String
            lists  List[]

            @@id([id, uTodo])
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Todo", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["id", "uTodo"]))?
            .assert_column("id", |col| {
                if api.is_sqlite() {
                    Ok(col)
                } else {
                    col.assert_auto_increments()
                }
            })
    })?;

    Ok(())
}

#[test_each_connector]
async fn migrating_a_unique_constraint_to_a_primary_key_works(api: &TestApi) -> TestResult {
    let dm = r#"
        model model1 {
            id              String        @id @default(cuid())
            a               String
            b               String
            c               String

            @@unique([a, b, c])

        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("model1", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["id"]))?
            .assert_index_on_columns(&["a", "b", "c"], |idx| idx.assert_is_unique())
    })?;

    api.insert("model1")
        .value("id", "the-id")
        .value("a", "the-a")
        .value("b", "the-b")
        .value("c", "the-c")
        .result_raw()
        .await?;

    let dm2 = r#"
        model model1 {
            a               String
            b               String
            c               String

            @@id([a, b, c])

        }
    "#;

    api.schema_push(dm2)
        .force(true)
        .send()
        .await?
        .assert_executable()?
        .assert_warnings(&["The migration will change the primary key for the `model1` table. If it partially fails, the table could be left without primary key constraint.".into(), "You are about to drop the column `id` on the `model1` table, which still contains 1 non-null values.".into()])?;

    api.assert_schema().await?.assert_table("model1", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["a", "b", "c"]))
    })?;

    Ok(())
}

#[test_each_connector]
async fn adding_multiple_optional_fields_to_an_existing_model_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let dm2 = r#"
        model Cat {
            id   Int @id
            name String?
            age  Int?
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table
            .assert_column("name", |col| col.assert_is_nullable())?
            .assert_column("age", |col| col.assert_is_nullable())
    })?;

    Ok(())
}
