use migration_engine_tests::*;
use sql_schema_describer::{ColumnArity, ColumnTypeFamily};

#[test_each_connector(tags("postgres"))]
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

    api.infer_apply(dm1).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_enum("CatMood", |r#enum| r#enum.assert_values(&["ANGRY", "CUDDLY", "HUNGRY"]))?;

    let dm2 = r#"
        model Cat {
            id String @id
            name String
        }
    "#;

    api.infer_apply(dm2).send().await?.assert_green()?;
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

// Reference for the tables created by PostGIS: https://postgis.net/docs/manual-1.4/ch04.html#id418599
#[test_each_connector(tags("postgres"))]
async fn existing_postgis_tables_must_not_be_migrated(api: &TestApi) -> TestResult {
    let create_spatial_ref_sys_table = "CREATE TABLE IF NOT EXISTS \"spatial_ref_sys\" ( id SERIAL PRIMARY KEY )";
    // The capitalized Geometry is intentional here, because we want the matching to be case-insensitive.
    let create_geometry_columns_table = "CREATE TABLE IF NOT EXiSTS \"Geometry_columns\" ( id SERIAL PRIMARY KEY )";

    api.database().execute_raw(create_spatial_ref_sys_table, &[]).await?;
    api.database().execute_raw(create_geometry_columns_table, &[]).await?;

    api.assert_schema()
        .await?
        .assert_has_table("spatial_ref_sys")?
        .assert_has_table("Geometry_columns")?;

    let schema = "";

    api.infer_apply(schema)
        .send()
        .await?
        .assert_green()?
        .assert_no_steps()?;

    api.assert_schema()
        .await?
        .assert_has_table("spatial_ref_sys")?
        .assert_has_table("Geometry_columns")?;

    Ok(())
}
