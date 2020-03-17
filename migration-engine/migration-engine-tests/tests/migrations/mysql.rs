use migration_engine_tests::sql::*;

/// We need to test this specifically for mysql, because foreign keys are indexes, and they are
/// inferred as both foreign key and index by the sql-schema-describer. We do not want to
/// create/delete a second index.
#[test_each_connector(tags("mysql"))]
async fn indexes_on_foreign_key_fields_are_not_created_twice(api: &TestApi) -> TestResult {
    let schema = r#"
        model Human {
            id String @id
            cat Cat @relation(references: [name])
        }

        model Cat {
            id String @id
            name String @unique
            humans Human[]
        }
    "#;

    api.infer_apply(schema).send().await?;

    let sql_schema = api
        .assert_schema()
        .await?
        .assert_table("Human", |table| {
            table
                .assert_foreign_keys_count(1)?
                .assert_fk_on_columns(&["cat"], |fk| fk.assert_references("Cat", &["name"]))?
                .assert_indexes_count(1)?
                .assert_index_on_columns(&["cat"], |idx| idx.assert_is_not_unique())
        })?
        .into_schema();

    // Test that after introspection, we do not migrate further.
    api.infer_apply(schema)
        .force(Some(true))
        .send_assert()
        .await?
        .assert_green()?
        .assert_no_steps()?;

    api.assert_schema().await?.assert_equals(&sql_schema)?;

    Ok(())
}

// We have to test this because one enum on MySQL can map to multiple enums in the database.
#[test_each_connector(tags("mysql"))]
async fn enum_creation_is_idempotent(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id String @id
            mood Mood
        }

        model Human {
            id String @id
            mood Mood
        }

        enum Mood {
            HAPPY
            HUNGRY
        }
    "#;

    api.infer_apply(dm1).send_assert().await?.assert_green()?;

    api.infer_apply(dm1)
        .send_assert()
        .await?
        .assert_green()?
        .assert_no_steps()?;

    Ok(())
}
