use migration_engine_tests::sql::*;

#[test_each_connector(tags("sqlite"))]
async fn sqlite_must_recreate_indexes(api: &TestApi) -> TestResult {
    // SQLite must go through a complicated migration procedure which requires dropping and recreating indexes. This test checks that.
    // We run them still against each connector.
    let dm1 = r#"
        model A {
            id Int @id
            field String @unique
        }
    "#;

    api.infer_apply(&dm1).send().await?;

    api.assert_schema().await?.assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    })?;

    let dm2 = r#"
        model A {
            id    Int    @id
            field String @unique
            other String
        }
    "#;

    api.infer_apply(&dm2).send().await?;

    api.assert_schema().await?.assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    })?;

    Ok(())
}

#[test_each_connector(tags("sqlite"))]
async fn sqlite_must_recreate_multi_field_indexes(api: &TestApi) -> TestResult {
    // SQLite must go through a complicated migration procedure which requires dropping and recreating indexes. This test checks that.
    // We run them still against each connector.
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
        table.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    })?;

    let dm2 = r#"
        model A {
            id    Int    @id
            field String
            secondField Int
            other String

            @@unique([field, secondField])
        }
    "#;

    api.infer_apply(&dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    })?;

    Ok(())
}
