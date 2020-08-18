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

    api.infer_apply(dm).send().await?.assert_green()?;

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
            pairidx Int
            pairname String
            pair Pair @relation(fields: [pairidx, pairname], references: [index, name])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Pair", |table| table.assert_has_no_pk())?
        .assert_table("PairMetadata", |table| {
            table
                .assert_pk(|pk| pk.assert_columns(&["id"]))?
                .assert_fk_on_columns(&["pairidx", "pairname"], |fk| {
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
            pweight Float
            pair Pair @relation(fields: [pweight], references: [weight])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Pair", |table| table.assert_has_no_pk())?
        .assert_table("PairMetadata", |table| {
            table
                .assert_pk(|pk| pk.assert_columns(&["id"]))?
                .assert_fk_on_columns(&["pweight"], |fk| fk.assert_references("Pair", &["weight"]))
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
        .send()
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

    if api.is_mysql() {
        api.infer_apply(dm).force(Some(true)).send().await?.assert_warnings(&["The migration will remove the values [hongry] on the enum `Cat_mood`. If these variants are still used in the database, the migration will fail.".into()])?;
        api.assert_schema()
            .await?
            .assert_enum("Cat_mood", |enm| enm.assert_values(&["ANGRY", "hongery"]))?;
    } else {
        api.infer_apply(dm).force(Some(true)).send().await?.assert_warnings(&["The migration will remove the values [hongry] on the enum `CatMood`. If these variants are still used in the database, the migration will fail.".into()])?;
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
        .send()
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
            nemesis_id String @id
            nemesis Dog @relation(fields: [nemesis_id], references: [id])
        }

        model Dog {
            id String @id
        }
    "##;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["nemesis_id"]))?
            .assert_fk_on_columns(&["nemesis_id"], |fk| fk.assert_references("Dog", &["id"]))
    })?;

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn multi_field_id_as_part_of_relation_must_work(api: &TestApi) -> TestResult {
    let dm = r##"
        model Cat {
            nemesis_name String
            nemesis_weight Int

            nemesis Dog @relation(fields: [nemesis_name, nemesis_weight], references: [name, weight])

            @@id([nemesis_name, nemesis_weight])
        }

        model Dog {
            name String
            weight Int

            @@id([name, weight])
        }
    "##;

    api.infer_apply(dm).send().await?.assert_green()?;

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
            nemesis_name String @map("dogname")
            nemesis_weight Int @map("dogweight")
            nemesis Dog @relation(fields: [nemesis_name, nemesis_weight], references: [name, weight])

            @@id([nemesis_name, nemesis_weight])
        }

        model Dog {
            name String
            weight Int

            @@id([name, weight])
        }
    "##;

    api.infer_apply(dm).send().await?.assert_green()?;

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
            chiid Int
            chic  String
            child Child  @relation(fields: [chiid, chic], references: [id, c])
            p     String

            @@unique([chiid, chic])
        }

        model Child {
            id        Int    @id
            c         String

            @@unique([id, c])
        }
    "##;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Parent", |table| {
        table.assert_index_on_columns(&["chiid", "chic"], |idx| idx.assert_is_unique())
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
          ufn String
          uln String
          user User @relation(fields: [ufn, uln], references: [firstName, lastName])

          @@index([ufn, uln])
        }
    "##;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("SpamList", |table| {
        table.assert_index_on_columns(&["ufn", "uln"], |idx| idx.assert_is_not_unique())
    })?;

    Ok(())
}

#[test_each_connector(tags("sql"))]
async fn dropping_mutually_referencing_tables_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
    model A {
        id Int @id
        b_id Int
        ab B @relation("AtoB", fields: [b_id], references: [id])
        c_id Int
        ac C @relation("AtoC", fields: [c_id], references: [id])
    }

    model B {
        id Int @id
        a_id Int
        ba A @relation("BtoA", fields: [a_id], references: [id])
        c_id Int
        bc C @relation("BtoC", fields: [c_id], references: [id])
    }

    model C {
        id Int @id
        a_id Int
        ca A @relation("CtoA", fields: [a_id], references: [id])
        b_id Int
        cb B @relation("CtoB", fields: [b_id], references: [id])
    }

    "#;

    api.infer_apply(dm1).send().await?.assert_green()?;
    api.assert_schema().await?.assert_tables_count(3)?;

    api.infer_apply("").send().await?.assert_green()?;
    api.assert_schema().await?.assert_tables_count(0)?;

    Ok(())
}
