use migration_engine_tests::sql::*;
use quaint::prelude::SqlFamily;
use sql_schema_describer::{ColumnArity, ColumnTypeFamily, ForeignKey, ForeignKeyAction};

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

    api.schema_push(dm1).send().await?.assert_green()?;

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

    api.schema_push(dm1).send().await?.assert_green()?;

    let result = api.describe_database().await?;

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
async fn specifying_a_db_name_for_an_inline_relation_must_work(api: &TestApi) -> TestResult {
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

    api.schema_push(dm1).send().await?.assert_green()?;

    let result = api.describe_database().await?;

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

    Ok(())
}

#[test_each_connector]
async fn adding_an_inline_relation_to_a_model_with_an_exotic_id_type(api: &TestApi) -> TestResult {
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

    api.schema_push(dm1).send().await?.assert_green()?;

    let result = api.describe_database().await?;

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

    Ok(())
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

    api.schema_push(dm1).send().await?.assert_green()?;

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

    api.schema_push(dm2).send().await?.assert_green()?;

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

    api.schema_push(dm1).send().await?.assert_green()?;

    let result = api.describe_database().await?;

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

    api.schema_push(dm2).send().await?.assert_green()?;

    let result = api.describe_database().await?;

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
