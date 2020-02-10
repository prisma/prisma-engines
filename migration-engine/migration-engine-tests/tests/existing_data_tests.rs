mod existing_data;

use migration_connector::MigrationWarning;
use migration_engine_tests::sql::*;
use pretty_assertions::assert_eq;
use quaint::ast::*;

#[test_each_connector]
async fn adding_a_required_field_if_there_is_data(api: &TestApi) {
    let dm = r#"
            model Test {
                id String @id @default(cuid())
            }

            enum MyEnum {
                B
                A
            }
        "#;
    api.infer_and_apply(&dm).await.sql_schema;

    let insert = Insert::single_into((api.schema_name(), "Test")).value("id", "test");
    api.database().query(insert.into()).await.unwrap();

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
    api.infer_and_apply(&dm).await;
}

#[test_each_connector]
async fn adding_a_required_field_must_use_the_default_value_for_migrations(api: &TestApi) -> TestResult {
    let dm = r#"
            model Test {
                id String @id @default(cuid())
            }

            enum MyEnum {
                B
                A
            }
        "#;

    api.infer_apply(&dm).send().await?;

    let conn = api.database();
    let insert = Insert::single_into((api.schema_name(), "Test")).value("id", "test");

    conn.query(insert.into()).await.unwrap();

    let dm = r#"
            model Test {
                id String @id @default(cuid())
                myint Int @default(1)
                myfloat Float @default(2)
                boolean Boolean @default(true)
                string String @default("test_string")
                dateTime DateTime
                enum MyEnum @default(A)
            }

            enum MyEnum {
                B
                A
            }
        "#;
    api.infer_and_apply(&dm).await;

    let query = Select::from_table(api.render_table_name("Test"))
        .column("myint")
        .column("string")
        .so_that("id".equals("test"));
    let result_set = conn.query(query.into()).await.unwrap();
    let row = result_set.into_iter().next().expect("query returned no results");
    assert_eq!(row["myint"].as_i64().unwrap(), 1);
    assert_eq!(row["string"].as_str().unwrap(), "test_string");

    Ok(())
}

#[test_each_connector]
async fn dropping_a_table_with_rows_should_warn(api: &TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
        }
    "#;
    let original_database_schema = api.infer_and_apply(&dm).await.sql_schema;

    let conn = api.database();
    let insert = Insert::single_into((api.schema_name(), "Test")).value("id", "test");

    conn.query(insert.into()).await.unwrap();

    let dm = "";

    let InferAndApplyOutput {
        migration_output,
        sql_schema: final_database_schema,
    } = api.infer_and_apply(&dm).await;

    // The schema should not change because the migration should not run if there are warnings
    // and the force flag isn't passed.
    assert_eq!(original_database_schema, final_database_schema);

    assert_eq!(
        migration_output.warnings,
        &[MigrationWarning {
            description: "You are about to drop the table `Test`, which is not empty (1 rows).".into()
        }]
    );
}

#[test_each_connector]
async fn dropping_a_column_with_non_null_values_should_warn(api: &TestApi) {
    let dm = r#"
            model Test {
                id String @id @default(cuid())
                puppiesCount Int?
            }
        "#;

    let original_database_schema = api.infer_and_apply(&dm).await.sql_schema;

    let insert = Insert::multi_into((api.schema_name(), "Test"), vec!["id", "puppiesCount"])
        .values(("a", 7))
        .values(("b", 8));

    api.database().query(insert.into()).await.unwrap();

    // Drop the `favouriteAnimal` column.
    let dm = r#"
            model Test {
                id String @id @default(cuid())
            }
        "#;

    let InferAndApplyOutput {
        migration_output,
        sql_schema: final_database_schema,
    } = api.infer_and_apply(&dm).await;

    // The schema should not change because the migration should not run if there are warnings
    // and the force flag isn't passed.
    assert_eq!(original_database_schema, final_database_schema);

    assert_eq!(
            migration_output.warnings,
            &[MigrationWarning {
                description: "You are about to drop the column `puppiesCount` on the `Test` table, which still contains 2 non-null values.".to_owned(),
            }]
        );
}

#[test_each_connector]
async fn altering_a_column_without_non_null_values_should_not_warn(api: &TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            puppiesCount Int?
        }
    "#;

    let original_database_schema = api.infer_and_apply(&dm).await.sql_schema;

    let insert = Insert::multi_into((api.schema_name(), "Test"), vec!["id"])
        .values(vec!["a"])
        .values(vec!["b"]);

    api.database().query(insert.into()).await.unwrap();

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            puppiesCount Float?
        }
    "#;

    let result = api.infer_and_apply(&dm2).await;
    let final_database_schema = &result.sql_schema;

    assert_ne!(&original_database_schema, final_database_schema);
    assert!(result.migration_output.warnings.is_empty());
}

#[test_each_connector]
async fn altering_a_column_with_non_null_values_should_warn(api: &TestApi) -> TestResult {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            age Int?
        }
    "#;

    api.infer_apply(&dm).send().await?;
    let original_database_schema = api.describe_database().await?;

    let insert = Insert::multi_into((api.schema_name(), "Test"), vec!["id", "age"])
        .values(("a", 12))
        .values(("b", 22));

    api.database().query(insert.into()).await.unwrap();

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            age Float?
        }
    "#;

    let migration_output = api.infer_apply(&dm2).send().await?;
    // The schema should not change because the migration should not run if there are warnings
    // and the force flag isn't passed.
    api.assert_schema().await?.assert_equals(&original_database_schema)?;

    assert_eq!(
        migration_output.warnings,
        &[MigrationWarning {
            description:
                "You are about to alter the column `age` on the `Test` table, which still contains 2 non-null values. \
                 The data in that column will be lost."
                    .to_owned()
        }]
    );

    let data = api.dump_table("Test").await?;
    assert_eq!(data.len(), 2);

    Ok(())
}

#[test_each_connector]
async fn column_defaults_can_safely_be_changed(api: &TestApi) -> TestResult {
    let combinations = &[
        ("Meow", Some("Cats"), None),
        ("Freedom", None, Some("Braveheart")),
        ("OutstandingMovies", Some("Cats"), Some("Braveheart")),
    ];

    for (model_name, first_default, second_default) in combinations {
        // Set up the initial schema
        {
            let dm1 = format!(
                r#"
                    model {} {{
                        id String @id
                        name String {}
                    }}
                "#,
                model_name,
                first_default
                    .map(|default| format!("@default(\"{}\")", default))
                    // As a temporary hack, columns always have defaults. Blocked on unexecutable migrations spec.
                    .unwrap_or_else(|| format!("@default(\"\")"))
            );

            api.infer_apply(&dm1).force(Some(true)).send().await?;

            api.assert_schema().await?.assert_table(model_name, |table| {
                table.assert_column("name", |column| {
                    column.assert_default(Some(first_default.unwrap_or("")))
                })
            })?;
        }

        // Insert data
        {
            let query = Insert::single_into(api.render_table_name(model_name)).value("id", "abc");

            api.database().query(query.into()).await?;

            let query = Insert::single_into(api.render_table_name(model_name))
                .value("id", "def")
                .value("name", "Waterworld");

            api.database().query(query.into()).await?;

            let data = api.dump_table(model_name).await?;
            let names: Vec<String> = data
                .into_iter()
                .filter_map(|row| row.get("name").and_then(|val| val.to_string()))
                .collect();
            // TODO: change this when the defaults hack is removed
            assert_eq!(&[first_default.unwrap_or(""), "Waterworld"], names.as_slice());
        }

        // Migrate
        {
            let dm2 = format!(
                r#"
                    model {} {{
                        id String @id
                        name String {}
                    }}
                "#,
                model_name,
                second_default
                    .map(|default| format!("@default(\"{}\")", default))
                    // As a temporary hack, columns always have defaults. Blocked on unexecutable migrations spec.
                    .unwrap_or_else(|| format!("@default(\"\")"))
            );

            let response = api.infer_apply(&dm2).send().await?;

            anyhow::ensure!(
                response.warnings.is_empty(),
                "Warnings should be empty. Got {:?}",
                response.warnings
            );
        }

        // Check that the data is still there
        {
            let data = api.dump_table(model_name).await?;
            let names: Vec<String> = data
                .into_iter()
                .filter_map(|row| row.get("name").and_then(|val| val.to_string()))
                .collect();
            assert_eq!(&[first_default.unwrap_or(""), "Waterworld"], names.as_slice());

            api.assert_schema().await?.assert_table(model_name, |table| {
                table.assert_column("name", |column| {
                    column.assert_default(Some(second_default.unwrap_or("")))
                })
            })?;
        }
    }

    Ok(())
}

#[test_each_connector]
async fn changing_a_column_from_required_to_optional_should_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            age Int @default(30)
        }
    "#;

    api.infer_apply(&dm).send().await?;
    let original_database_schema = api.describe_database().await?;

    let insert = Insert::multi_into((api.schema_name(), "Test"), vec!["id", "age"])
        .values(("a", 12))
        .values(("b", 22));

    api.database().query(insert.into()).await.unwrap();

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            age Int? @default(30)
        }
    "#;

    let migration_output = api.infer_apply(&dm2).send().await?;

    // On MySQL we can't safely restate the type in a CHANGE clause, so this change is still destructive.
    if api.is_mysql() {
        anyhow::ensure!(
            migration_output.warnings.len() == 1,
            "Migration warnings should have one warning on mysql. Got {:#?}",
            migration_output.warnings
        );

        assert_eq!(
            migration_output.warnings.get(0).unwrap().description,
            "You are about to alter the column `age` on the `Test` table, which still contains 2 non-null values. The data in that column will be lost.",
        );

        api.assert_schema().await?.assert_equals(&original_database_schema)?;
    } else {
        // On other databases, the migration should be successful.
        anyhow::ensure!(
            migration_output.warnings.is_empty(),
            "Migration warnings should be empty. Got {:#?}",
            migration_output.warnings
        );

        api.assert_schema().await?.assert_ne(&original_database_schema)?;
    }

    // Check that no data was lost.
    {
        let data = api.dump_table("Test").await?;
        assert_eq!(data.len(), 2);
        let ages: Vec<i64> = data
            .into_iter()
            .map(|row| row.get("age").unwrap().as_i64().unwrap())
            .collect();

        assert_eq!(ages, &[12, 22]);
    }

    Ok(())
}

#[test_each_connector]
async fn changing_a_column_from_optional_to_required_must_warn(api: &TestApi) -> TestResult {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
            age Int?
        }
    "#;

    api.infer_apply(&dm).send().await?;
    let original_database_schema = api.describe_database().await?;

    let insert = Insert::multi_into((api.schema_name(), "Test"), vec!["id", "age"])
        .values(("a", 12))
        .values(("b", 22));

    api.database().query(insert.into()).await.unwrap();

    let dm2 = r#"
        model Test {
            id String @id @default(cuid())
            age Int @default(30)
        }
    "#;

    let migration_output = api.infer_apply(&dm2).send().await?;

    // The schema should not change because the migration should not run if there are warnings
    // and the force flag isn't passed.
    api.assert_schema().await?.assert_equals(&original_database_schema)?;

    assert_eq!(
        migration_output.warnings,
        &[MigrationWarning {
            description:
                "You are about to alter the column `age` on the `Test` table, which still contains 2 non-null values. \
                 The data in that column will be lost."
                    .to_owned()
        }]
    );

    // Check that no data was lost.
    {
        let data = api.dump_table("Test").await?;
        assert_eq!(data.len(), 2);
        let ages: Vec<i64> = data
            .into_iter()
            .map(|row| row.get("age").unwrap().as_i64().unwrap())
            .collect();

        assert_eq!(ages, &[12, 22]);
    }

    Ok(())
}

#[test_each_connector]
async fn dropping_a_table_referenced_by_foreign_keys_must_work(api: &TestApi) -> TestResult {
    use quaint::ast::*;

    let dm1 = r#"
        model Category {
            id Int @id
            name String
        }

        model Recipe {
            id Int @id
            category Category
        }
    "#;

    let sql_schema = api.infer_and_apply(&dm1).await.sql_schema;
    assert!(sql_schema.table("Category").is_ok());

    let id: i32 = 1;

    let insert = Insert::single_into(api.render_table_name("Category"))
        .value("name", "desserts")
        .value("id", id);
    api.database().query(insert.into()).await?;

    let insert = Insert::single_into(api.render_table_name("Recipe"))
        .value("category", id)
        .value("id", id);
    api.database().query(insert.into()).await?;

    let fk = sql_schema.table_bang("Recipe").foreign_keys.get(0).unwrap();
    assert_eq!(fk.referenced_table, "Category");

    let dm2 = r#"
        model Recipe {
            id Int @id
        }
    "#;

    api.infer_apply(dm2).force(Some(true)).send().await?;
    let sql_schema = api.describe_database().await.unwrap();

    assert!(sql_schema.table("Category").is_err());
    assert!(sql_schema.table_bang("Recipe").foreign_keys.is_empty());

    Ok(())
}

#[test_each_connector]
async fn string_columns_do_not_get_arbitrarily_migrated(api: &TestApi) -> TestResult {
    use quaint::ast::*;

    let dm1 = r#"
        model User {
            id           String  @default(cuid()) @id
            name         String?
            email        String  @unique
            kindle_email String? @unique
        }
    "#;

    api.infer_apply(dm1).send().await?;

    let insert = Insert::single_into(api.render_table_name("User"))
        .value("id", "the-id")
        .value("name", "George")
        .value("email", "george@prisma.io")
        .value("kindle_email", "george+kindle@prisma.io");

    api.database().query(insert.into()).await?;

    let dm2 = r#"
        model User {
            id           String  @default(cuid()) @id
            name         String?
            email        String  @unique
            kindle_email String? @unique
            count        Int     @default(0)
        }
    "#;

    let output = api.infer_apply(dm2).send().await?;

    assert!(output.warnings.is_empty());

    // Check that the string values are still there.
    let select = Select::from_table(api.render_table_name("User"))
        .column("name")
        .column("kindle_email")
        .column("email");

    let counts = api.database().query(select.into()).await?;

    let row = counts.get(0).unwrap();

    assert_eq!(row.get("name").unwrap().as_str().unwrap(), "George");
    assert_eq!(
        row.get("kindle_email").unwrap().as_str().unwrap(),
        "george+kindle@prisma.io"
    );
    assert_eq!(row.get("email").unwrap().as_str().unwrap(), "george@prisma.io");

    Ok(())
}

#[test_each_connector]
async fn altering_the_type_of_a_column_in_an_empty_table_should_not_warn(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs Int
        }
    "#;

    api.infer_apply(dm1).send().await?;

    let dm2 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs String
        }
    "#;

    let response = api.infer_apply(dm2).send().await?;

    assert!(response.warnings.is_empty());

    api.assert_schema()
        .await?
        .assert_table("User", |table| {
            table.assert_column("dogs", |col| col.assert_type_is_string()?.assert_is_required())
        })
        .map(drop)
}

#[test_each_connector]
async fn making_a_column_required_in_an_empty_table_should_not_warn(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs Int?
        }
    "#;

    api.infer_apply(dm1).send().await?;

    let dm2 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs Int
        }
    "#;

    let response = api.infer_apply(dm2).send().await?;

    assert!(response.warnings.is_empty());

    api.assert_schema()
        .await?
        .assert_table("User", |table| {
            table.assert_column("dogs", |col| col.assert_type_is_int()?.assert_is_required())
        })
        .map(drop)
}

#[test_each_connector]
async fn altering_the_type_of_a_column_in_a_non_empty_table_always_warns(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model User {
            id String @id @default(cuid())
            name String
            dogs Int
        }
    "#;

    api.infer_apply(dm1).send().await?;

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

    let response = api.infer_apply(dm2).send().await?;

    assert_eq!(
        response.warnings,
        &[MigrationWarning {
            // TODO: the message should say that altering the type of a column is not guaranteed to preserve the data, but the database is going to do its best.
            // Also think about timeouts.
            description: "You are about to alter the column `dogs` on the `User` table, which still contains 1 non-null values. The data in that column will be lost.".to_owned()
        }]
    );

    let rows = api.select("User").column("dogs").send_debug().await?;
    assert_eq!(rows, &[["Integer(7)"]]);

    api.assert_schema()
        .await?
        .assert_table("User", |table| {
            table.assert_column("dogs", |col| col.assert_type_is_int()?.assert_is_required())
        })
        .map(drop)
}

#[test_each_connector(ignore("mysql"))]
async fn migrating_a_required_column_from_int_to_string_should_warn_and_cast(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            serialNumber Int
        }
    "#;

    api.infer_apply(dm1).send().await?;

    api.insert("Test")
        .value("id", "abcd")
        .value("serialNumber", 47i64)
        .result_raw()
        .await?;

    let test = api.dump_table("Test").await?;
    let first_row = test.get(0).unwrap();
    assert_eq!(
        format!("{:?} {:?}", first_row.get("id"), first_row.get("serialNumber")),
        r#"Some(Text("abcd")) Some(Integer(47))"#
    );

    let original_schema = api.assert_schema().await?.into_schema();

    let dm2 = r#"
        model Test {
            id String @id
            serialNumber String
        }
    "#;

    let expected_warning = MigrationWarning {
        description: "You are about to alter the column `serialNumber` on the `Test` table, which still contains 1 non-null values. The data in that column will be lost.".to_owned(),
    };

    // Apply once without forcing
    {
        let result = api.infer_apply(dm2).send().await?;

        assert_eq!(result.warnings, &[expected_warning.clone()]);

        api.assert_schema().await?.assert_equals(&original_schema)?;
    }

    // Force apply
    {
        let result = api.infer_apply(dm2).force(Some(true)).send().await?;

        assert_eq!(result.warnings, &[expected_warning]);

        api.assert_schema().await?.assert_table("Test", |table| {
            table.assert_column("serialNumber", |col| col.assert_type_is_string())
        })?;

        let test = api.dump_table("Test").await?;
        let first_row = test.get(0).unwrap();
        assert_eq!(
            format!("{:?} {:?}", first_row.get("id"), first_row.get("serialNumber")),
            r#"Some(Text("abcd")) Some(Text("47"))"#
        );
    }

    Ok(())
}
