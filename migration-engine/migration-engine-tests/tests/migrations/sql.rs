use migration_engine_tests::sql::*;

#[test_each_connector(tags("sql"))]
async fn creating_tables_without_primary_key_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Pair {
            index Int
            name String
            weight Float

            @@unique([index, name])
        }
    "#;

    api.infer_apply(dm).send_assert().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Pair", |table| {
        table
            .assert_has_no_pk()?
            .assert_index_on_columns(&["index", "name"], |idx| idx.assert_is_unique())
    })?;

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn relations_to_models_without_a_primary_key_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Pair {
            index Int
            name String
            weight Float

            @@unique([index, name])
        }

        model PairMetadata {
            id String @id
            pair Pair
        }
    "#;

    api.infer_apply(dm).send_assert().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Pair", |table| table.assert_has_no_pk())?
        .assert_table("PairMetadata", |table| {
            table
                .assert_pk(|pk| pk.assert_columns(&["id"]))?
                .assert_fk_on_columns(&["pair_index", "pair_name"], |fk| {
                    fk.assert_references("Pair", &["index", "name"])
                })
        })?;

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn relations_to_models_with_no_pk_and_a_single_unique_required_field_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Pair {
            index Int
            name String
            weight Float @unique
        }

        model PairMetadata {
            id String @id
            pair Pair
        }
    "#;

    api.infer_apply(dm).send_assert().await?.assert_green()?;

    api.assert_schema()
        .await?
        .debug_print()
        .assert_table("Pair", |table| table.assert_has_no_pk())?
        .assert_table("PairMetadata", |table| {
            table
                .assert_pk(|pk| pk.assert_columns(&["id"]))?
                .assert_fk_on_columns(&["pair"], |fk| fk.assert_references("Pair", &["weight"]))
        })?;

    Ok(())
}

#[test_each_connector(capabilities("enums"), tags("sql"))]
async fn enum_value_with_database_names_must_work(api: &TestApi) -> TestResult {
    let dm = r##"
        model Cat {
            id String @id
            mood CatMood
        }

        enum CatMood {
            ANGRY
            HUNGRY @map("hongry")
        }
    "##;

    api.infer_apply(dm)
        .migration_id(Some("initial"))
        .send_assert()
        .await?
        .assert_green()?;

    if api.is_mysql() {
        api.assert_schema()
            .await?
            .assert_enum("Cat_mood", |enm| enm.assert_values(&["ANGRY", "hongry"]))?;
    } else {
        api.assert_schema()
            .await?
            .assert_enum("CatMood", |enm| enm.assert_values(&["ANGRY", "hongry"]))?;
    }

    let dm = r##"
        model Cat {
            id String @id
            mood CatMood
        }

        enum CatMood {
            ANGRY
            HUNGRY @map("hongery")
        }
    "##;

    api.infer_apply(dm).send_assert().await?.assert_green()?;

    if api.is_mysql() {
        api.assert_schema()
            .await?
            .assert_enum("Cat_mood", |enm| enm.assert_values(&["ANGRY", "hongery"]))?;
    } else {
        api.assert_schema()
            .await?
            .assert_enum("CatMood", |enm| enm.assert_values(&["ANGRY", "hongery"]))?;
    }

    Ok(())
}
