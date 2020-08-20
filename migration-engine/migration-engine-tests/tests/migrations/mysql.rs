use migration_engine_tests::sql::*;

/// We need to test this specifically for mysql, because foreign keys are indexes, and they are
/// inferred as both foreign key and index by the sql-schema-describer. We do not want to
/// create/delete a second index.
#[test_each_connector(tags("mysql"))]
async fn indexes_on_foreign_key_fields_are_not_created_twice(api: &TestApi) -> TestResult {
    let schema = r#"
        model Human {
            id String @id
            catname String
            cat_rel Cat @relation(fields: [catname], references: [name])
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
                .assert_fk_on_columns(&["catname"], |fk| fk.assert_references("Cat", &["name"]))?
                .assert_indexes_count(1)?
                .assert_index_on_columns(&["catname"], |idx| idx.assert_is_not_unique())
        })?
        .into_schema();

    // Test that after introspection, we do not migrate further.
    api.infer_apply(schema)
        .force(Some(true))
        .send()
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

    api.infer_apply(dm1).send().await?.assert_green()?;

    api.infer_apply(dm1).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn enums_work_when_table_name_is_remapped(api: &TestApi) -> TestResult {
    let schema = r#"
    model User {
        id         String     @default(uuid()) @id
        status     UserStatus @map("currentStatus___")

        @@map("users")
    }

    enum UserStatus {
        CONFIRMED
        CANCELED
        BLOCKED
    }
    "#;

    api.infer_apply(schema).send().await?.assert_green()?;

    Ok(())
}

#[test_each_connector(tags("mysql"), log = "debug,sql_schema_describer=info")]
async fn arity_of_enum_columns_can_be_changed(api: &TestApi) -> TestResult {
    let dm1 = r#"
        enum Color {
            RED
            GREEN
            BLUE
        }

        model A {
            id              Int @id
            primaryColor    Color
            secondaryColor  Color?
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_required())?
            .assert_column("secondaryColor", |col| col.assert_is_nullable())
    })?;

    let dm2 = r#"
        enum Color {
            RED
            GREEN
            BLUE
        }

        model A {
            id              Int @id
            primaryColor    Color?
            secondaryColor  Color
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_nullable())?
            .assert_column("secondaryColor", |col| col.assert_is_required())
    })?;

    Ok(())
}
