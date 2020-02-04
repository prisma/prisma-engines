use migration_engine_tests::*;
use sql_schema_describer::{ColumnArity, ColumnTypeFamily};

#[test_each_connector(starts_with = "postgres")]
async fn enums_can_be_dropped_on_postgres(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id String @id
            name String
            mood CatMood
        }

        enum CatMood {
            ANGRY
            HUNGRY
            CUDDLY
        }
    "#;

    api.infer_apply(dm1).send_assert().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_enum("CatMood", |r#enum| r#enum.assert_values(&["ANGRY", "CUDDLY", "HUNGRY"]))?;

    let dm2 = r#"
        model Cat {
            id String @id
            name String
        }
    "#;

    api.infer_apply(dm2).send_assert().await?.assert_green()?;
    api.assert_schema().await?.assert_has_no_enum("CatMood")?;

    Ok(())
}

#[test_each_connector(capabilities("scalar_lists"))]
async fn adding_a_scalar_list_for_a_model_with_id_type_int_must_work(api: &TestApi) {
    let dm1 = r#"
        datasource pg {
            provider = "postgres"
            url = "postgres://localhost:5432"
        }

        model A {
            id Int @id
            strings String[]
            enums Status[]
        }

        enum Status {
            OK
            ERROR
        }
    "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;

    let table_for_a = result.table_bang("A");
    let string_column = table_for_a.column_bang("strings");
    assert_eq!(string_column.tpe.family, ColumnTypeFamily::String);
    assert_eq!(string_column.tpe.arity, ColumnArity::List);

    let enum_column = table_for_a.column_bang("enums");
    assert_eq!(enum_column.tpe.family, ColumnTypeFamily::Enum("Status".to_owned()));
    assert_eq!(enum_column.tpe.arity, ColumnArity::List);
}
