use migration_engine_tests::sql::*;
use std::borrow::Cow;

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

#[derive(serde::Deserialize, Debug, PartialEq)]
struct Cat<'a> {
    id: Cow<'a, str>,
    mood: Cow<'a, str>,
    #[serde(rename = "previousMood")]
    previous_mood: Cow<'a, str>,
}

#[test_each_connector(capabilities("enums"), tags("sql"))]
async fn enum_defaults_must_work(api: &TestApi) -> TestResult {
    let dm = r##"
        model Cat {
            id String @id
            mood CatMood @default(HUNGRY)
            previousMood CatMood @default(ANGRY)
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

    let insert = quaint::ast::Insert::single_into(api.render_table_name("Cat")).value("id", "the-id");
    api.database().execute(insert.into()).await?;

    let record = api
        .database()
        .query(
            quaint::ast::Select::from_table(api.render_table_name("Cat"))
                .column("id")
                .column("mood")
                .column("previousMood")
                .into(),
        )
        .await?;

    let cat: Cat = quaint::serde::from_row(record.into_single()?)?;

    let expected_cat = Cat {
        id: "the-id".into(),
        mood: "hongry".into(),
        previous_mood: "ANGRY".into(),
    };

    assert_eq!(cat, expected_cat);

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn id_as_part_of_relation_must_work(api: &TestApi) -> TestResult {
    let dm = r##"
        model Cat {
            nemesis Dog @id
        }

        model Dog {
            id String @id
        }
    "##;

    api.infer_apply(dm).send_assert().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["nemesis"]))?
            .assert_fk_on_columns(&["nemesis"], |fk| fk.assert_references("Dog", &["id"]))
    })?;

    Ok(())
}

#[test_each_connector(tags("sql"), log = "debug")]
async fn multi_field_id_as_part_of_relation_must_work(api: &TestApi) -> TestResult {
    let dm = r##"
        model Cat {
            nemesis Dog @id
        }

        model Dog {
            name String
            weight Int

            @@id([name, weight])
        }
    "##;

    api.infer_apply(dm).send_assert().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["nemesis_name", "nemesis_weight"]))?
            .assert_fk_on_columns(&["nemesis_name", "nemesis_weight"], |fk| {
                fk.assert_references("Dog", &["name", "weight"])
            })
    })?;

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn remapped_multi_field_id_as_part_of_relation_must_work(api: &TestApi) -> TestResult {
    let dm = r##"
        model Cat {
            nemesis Dog @map(["dogname", "dogweight"]) @id
        }

        model Dog {
            name String
            weight Int

            @@id([name, weight])
        }
    "##;

    api.infer_apply(dm).send_assert().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["dogname", "dogweight"]))?
            .assert_fk_on_columns(&["dogname", "dogweight"], |fk| {
                fk.assert_references("Dog", &["name", "weight"])
            })
    })?;

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn unique_constraints_on_composite_relation_fields(api: &TestApi) -> TestResult {
    let dm = r##"
        model Parent {
            id    Int    @id
            child Child  @relation(references: [id, c]) @unique
            p     String
        }

        model Child {
            id     Int    @id
            c      String
            parent Parent

            @@unique([id, c])
        }
    "##;

    api.infer_apply(dm).send_assert().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Parent", |table| {
        table.assert_index_on_columns(&["child_id", "child_c"], |idx| idx.assert_is_unique())
    })?;

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn indexes_on_composite_relation_fields(api: &TestApi) -> TestResult {
    let dm = r##"
        model User {
          id                  Int       @id
          firstName           String
          lastName            String

          @@unique([firstName, lastName])
        }

        model SpamList {
          id   Int  @id
          user User @relation(references: [firstName, lastName])

          @@index([user])
        }
    "##;

    api.infer_apply(dm).send_assert().await?.assert_green()?;

    api.assert_schema().await?.assert_table("SpamList", |table| {
        table.assert_index_on_columns(&["user_firstName", "user_lastName"], |idx| idx.assert_is_not_unique())
    })?;

    Ok(())
}
