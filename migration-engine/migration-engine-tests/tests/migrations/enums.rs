use migration_engine_tests::sql::*;

#[test_each_connector(capabilities("enums"))]
async fn an_enum_can_be_turned_into_a_model(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
            HAPPY
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let enum_name = if api.sql_family().is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    #[allow(clippy::redundant_closure)]
    api.assert_schema().await?.assert_enum(enum_name, |enm| Ok(enm))?;

    let dm2 = r#"
        model Cat {
            id Int @id
            moodId Int
            mood CatMood @relation(fields: [moodId], references: [id])
        }

        model CatMood {
            id Int @id
            description String
            biteRisk Int
            c        Cat[]
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Cat", |table| {
            table.assert_columns_count(2)?.assert_column("moodId", Ok)
        })?
        .assert_table("CatMood", |table| table.assert_column_count(3))?
        .assert_has_no_enum("CatMood")?;

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn variants_can_be_added_to_an_existing_enum(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let enum_name = if api.sql_family().is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    api.assert_schema()
        .await?
        .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY"]))?;

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
            HAPPY
            JOYJOY
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY", "HAPPY", "JOYJOY"]))?;

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn variants_can_be_removed_from_an_existing_enum(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    let enum_name = if api.sql_family().is_mysql() {
        "Cat_mood"
    } else {
        "CatMood"
    };

    api.assert_schema()
        .await?
        .assert_enum(enum_name, |enm| enm.assert_values(&["HAPPY", "HUNGRY"]))?;

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HUNGRY
        }
    "#;

    api.schema_push(dm2)
        .force(true)
        .send()
        .await?
        .assert_warnings(&[format!("The migration will remove the values [HAPPY] on the enum `{}`. If these variants are still used in the database, the migration will fail.", enum_name).into()])?
        .assert_executable()?;

    api.assert_schema()
        .await?
        .assert_enum(enum_name, |enm| enm.assert_values(&["HUNGRY"]))?;

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn models_with_enum_values_can_be_dropped(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_tables_count(1)?;

    api.insert("Cat")
        .value("id", 1)
        .value("mood", "HAPPY")
        .result_raw()
        .await?;

    api.schema_push("")
        .force(true)
        .send()
        .await?
        .assert_executable()?
        .assert_warnings(&["You are about to drop the `Cat` table, which is not empty (1 rows).".into()])?;

    api.assert_schema().await?.assert_tables_count(0)?;

    Ok(())
}

#[test_each_connector(log = "debug", capabilities("enums"))]
async fn enums_used_in_default_can_be_changed(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_tables_count(1)?;

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood @default(HAPPY)
        }

        enum CatMood {
            HAPPY
            ANGRY
        }
    "#;

    api.schema_push(dm2)
        .force(true)
        .send()
        .await?
        .assert_executable()?
        .assert_warnings(&["The migration will remove the values [HUNGRY] on the enum `Cat_mood`. If these variants are still used in the database, the migration will fail.".into()])?;

    api.assert_schema().await?.assert_tables_count(1)?;

    Ok(())
}
