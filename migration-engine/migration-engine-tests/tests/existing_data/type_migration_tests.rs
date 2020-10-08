use crate::*;
use pretty_assertions::assert_eq;

#[test_each_connector]
async fn altering_the_type_of_a_column_in_a_non_empty_table_always_warns(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs Int
        }
    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;

    let insert = quaint::ast::Insert::single_into(api.render_table_name("User"))
        .value("id", "abc")
        .value("name", "Shinzo")
        .value("dogs", 7);

    api.database().query(insert.into()).await?;

    let dm2 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs String
        }
    "#;

    api.infer_apply(dm2).send().await?.assert_warnings(&[
        // TODO: the message should say that altering the type of a column is not guaranteed to preserve the data, but the database is going to do its best.
        // Also think about timeouts.
        "You are about to alter the column `dogs` on the `User` table, which still contains 1 non-null values. The data in that column could be lost.".into()
    ])?;

    let rows = api.select("User").column("dogs").send_debug().await?;
    assert_eq!(rows, &[["Integer(Some(7))"]]);

    api.assert_schema().await?.assert_table("User", |table| {
        table.assert_column("dogs", |col| col.assert_type_is_int()?.assert_is_required())
    })?;

    Ok(())
}

#[test_each_connector(ignore("mysql"))]
async fn migrating_a_required_column_from_int_to_string_should_warn_and_cast(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            serialNumber Int
        }
    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;

    api.insert("Test")
        .value("id", "abcd")
        .value("serialNumber", 47i64)
        .result_raw()
        .await?;

    let test = api.dump_table("Test").await?;
    let first_row = test.get(0).unwrap();
    assert_eq!(
        format!("{:?} {:?}", first_row.get("id"), first_row.get("serialNumber")),
        r#"Some(Text(Some("abcd"))) Some(Integer(Some(47)))"#
    );

    let original_schema = api.assert_schema().await?.into_schema();

    let dm2 = r#"
        model Test {
            id String @id
            serialNumber String
        }
    "#;

    let expected_warning = "You are about to alter the column `serialNumber` on the `Test` table, which still contains 1 non-null values. The data in that column could be lost.";

    // Apply once without forcing
    {
        api.infer_apply(dm2)
            .send()
            .await?
            .assert_warnings(&[expected_warning.into()])?;
        api.assert_schema().await?.assert_equals(&original_schema)?;
    }

    // Force apply
    {
        api.infer_apply(dm2)
            .force(Some(true))
            .send()
            .await?
            .assert_warnings(&[expected_warning.into()])?;

        api.assert_schema().await?.assert_table("Test", |table| {
            table.assert_column("serialNumber", |col| col.assert_type_is_string())
        })?;

        let test = api.dump_table("Test").await?;
        let first_row = test.get(0).unwrap();
        assert_eq!(
            format!("{:?} {:?}", first_row.get("id"), first_row.get("serialNumber")),
            r#"Some(Text(Some("abcd"))) Some(Text(Some("47")))"#
        );
    }

    Ok(())
}

#[test_each_connector(capabilities("scalar_lists"))]
async fn changing_an_array_column_to_scalar_must_warn(api: &TestApi) -> TestResult {
    let datasource_block = api.datasource();

    let dm1 = format!(
        r#"
        {datasource_block}

        model Film {{
            id String @id
            mainProtagonist String[]
        }}
        "#,
        datasource_block = datasource_block,
    );

    api.infer_apply(&dm1).send().await?.assert_green()?;

    api.insert("Film")
        .value("id", "film1")
        .value(
            "mainProtagonist",
            Value::Array(Some(vec!["giant shark".into(), "jason statham".into()])),
        )
        // .value("mainProtagonist", Value::array(vec!["giant shark", "jason statham"]))
        .result_raw()
        .await?;

    let dm2 = format!(
        r#"
            {datasource_block}

            model Film {{
                id String @id
                mainProtagonist String
            }}
            "#,
        datasource_block = datasource_block,
    );

    api.infer_apply(&dm2)
        .force(Some(true))
        .send()
        .await?
        .assert_executable()?
        .assert_no_error()?
        .assert_warnings(&["You are about to alter the column `mainProtagonist` on the `Film` table, which still contains 1 non-null values. The data in that column could be lost.".into()])?;

    api.assert_schema().await?.assert_table("Film", |table| {
        table.assert_column("mainProtagonist", |column| column.assert_is_required())
    })?;

    let rows = api.select("Film").column("id").column("mainProtagonist").send().await?;

    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| row.into_iter().collect::<Vec<_>>())
        .collect();

    assert_eq!(
        rows,
        &[&["film1".into(), "{\"giant shark\",\"jason statham\"}".into()]] // the array got cast ot a string by postgres
    );

    Ok(())
}

#[test_each_connector(capabilities("scalar_lists"))]
async fn changing_a_scalar_column_to_an_array_is_unexecutable(api: &TestApi) -> TestResult {
    let datasource_block = api.datasource();

    let dm1 = format!(
        r#"
        {datasource_block}

        model Film {{
            id String @id
            mainProtagonist String
        }}
        "#,
        datasource_block = datasource_block,
    );

    api.infer_apply(&dm1).send().await?.assert_green()?;

    api.insert("Film")
        .value("id", "film1")
        .value("mainProtagonist", "left shark")
        // .value("mainProtagonist", Value::array(vec!["giant shark", "jason statham"]))
        .result_raw()
        .await?;

    let dm2 = format!(
        r#"
            {datasource_block}

            model Film {{
                id String @id
                mainProtagonist String[]
            }}
            "#,
        datasource_block = datasource_block,
    );

    api.infer_apply(&dm2)
        .send()
        .await?
        .assert_unexecutable(&[
            "Changed the column `mainProtagonist` on the `Film` table from a scalar field to a list field. There are 1 existing non-null values in that column, this migration step cannot be executed.".into(),
        ])?
        .assert_no_warning()?
        .assert_no_error()?;

    api.assert_schema().await?.assert_table("Film", |table| {
        table.assert_column("mainProtagonist", |column| column.assert_is_required())
    })?;

    let rows = api.select("Film").column("id").column("mainProtagonist").send().await?;

    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| row.into_iter().collect::<Vec<_>>())
        .collect();

    assert_eq!(rows, &[&["film1".into(), Value::text("left shark")]]);

    Ok(())
}
