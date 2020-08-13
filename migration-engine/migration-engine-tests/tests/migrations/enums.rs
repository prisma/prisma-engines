use migration_engine_tests::sql::*;

#[test_each_connector(capabilities("enums"), log = "debug,sql_schema_describer=info")]
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
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Cat", |table| {
            table.assert_columns_count(2)?.assert_column("moodId", |col| Ok(col))
        })?
        .assert_table("CatMood", |table| table.assert_column_count(3))?
        .assert_has_no_enum("CatMood")?;

    Ok(())
}
