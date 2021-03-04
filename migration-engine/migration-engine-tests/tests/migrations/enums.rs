use migration_engine_tests::sql::*;

const BASIC_ENUM_DM: &str = r#"
model Cat {
    id Int @id
    mood CatMood
}

enum CatMood {
    HAPPY
    HUNGRY
}
"#;

#[test_each_connector(capabilities("enums"))]
async fn an_enum_can_be_turned_into_a_model(api: &TestApi) -> TestResult {
    api.schema_push(BASIC_ENUM_DM).send().await?.assert_green()?;

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
    api.schema_push(BASIC_ENUM_DM).send().await?.assert_green()?;

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

#[test_each_connector(capabilities("enums"))]
async fn enum_field_to_string_field_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            mood CatMood?
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("mood", |col| col.assert_type_is_enum())
    })?;

    api.insert("Cat")
        .value("id", 1)
        .value("mood", "HAPPY")
        .result_raw()
        .await?;

    let dm2 = r#"
        model Cat {
            id      Int @id
            mood    String?
        }
    "#;

    api.schema_push(dm2).force(true).send().await?.assert_executable()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("mood", |col| col.assert_type_is_string())
    })?;

    Ok(())
}

#[test_each_connector(capabilities("enums"))]
async fn string_field_to_enum_field_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id      Int @id
            mood    String?
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("mood", |col| col.assert_type_is_string())
    })?;

    api.insert("Cat")
        .value("id", 1)
        .value("mood", "HAPPY")
        .result_raw()
        .await?;

    let dm2 = r#"
        model Cat {
            id Int @id
            mood CatMood?
        }

        enum CatMood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push(dm2).force(true).send().await?.assert_executable()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("mood", |col| col.assert_type_is_enum())
    })?;

    Ok(())
}
