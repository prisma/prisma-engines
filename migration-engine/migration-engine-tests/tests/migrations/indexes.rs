use migration_engine_tests::sql::*;
use sql_schema_describer::IndexType;

#[test_each_connector]
async fn index_on_compound_relation_fields_must_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model User {
            id String @id
            email String
            name String

            @@unique([email, name])
        }

        model Post {
            id String @id
            authorEmail String
            authorName String
            author User @relation(fields: [authorEmail, authorName], references: [email, name])

            @@index([authorEmail, authorName], name: "testIndex")
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |table| {
        table
            .assert_has_column("authorName")?
            .assert_has_column("authorEmail")?
            .assert_index_on_columns(&["authorEmail", "authorName"], |idx| idx.assert_name("testIndex"))
    })?;

    Ok(())
}

#[test_each_connector]
async fn index_settings_must_be_migrated(api: &TestApi) -> TestResult {
    let dm = r#"
        model Test {
            id String @id
            name String
            followersCount Int

            @@index([name, followersCount], name: "nameAndFollowers")
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["name", "followersCount"], |idx| {
                idx.assert_is_not_unique()?.assert_name("nameAndFollowers")
            })
    })?;

    let dm2 = r#"
        model Test {
            id String @id
            name String
            followersCount Int

            @@unique([name, followersCount], name: "nameAndFollowers")
        }
    "#;

    api.infer_apply(dm2)
        .force(Some(true))
        .send()
        .await?
        .assert_warnings(&["The migration will add a unique constraint covering the columns `[name,followersCount]` on the table `Test`. If there are existing duplicate values, the migration will fail.".into()])?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["name", "followersCount"], |idx| {
                idx.assert_is_unique()?.assert_name("nameAndFollowers")
            })
    })?;

    Ok(())
}

#[test_each_connector]
async fn unique_directive_on_required_one_to_one_relation_creates_one_index(api: &TestApi) -> TestResult {
    // We want to test that only one index is created, because of the implicit unique index on
    // required 1:1 relations.

    let dm = r#"
        model Cat {
            id Int @id
            boxId Int @unique
            box Box @relation(fields: [boxId], references: [id])
        }

        model Box {
            id Int @id
            cat Cat
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("Cat", |table| table.assert_indexes_count(1))?;

    Ok(())
}

#[test_each_connector]
async fn one_to_many_self_relations_do_not_create_a_unique_index(api: &TestApi) -> TestResult {
    let dm = r#"
        model Location {
            id        String      @id @default(cuid())
            parent    Location?   @relation("LocationToLocation_parent", fields:[parentId], references: [id])
            parentId  String?     @map("parent")
            children  Location[]  @relation("LocationToLocation_parent")
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    if api.is_mysql() {
        // MySQL creates an index for the FK.
        api.assert_schema().await?.assert_table("Location", |t| {
            t.assert_indexes_count(1)?
                .assert_index_on_columns(&["parent"], |idx| idx.assert_is_not_unique())
        })?;
    } else {
        api.assert_schema()
            .await?
            .assert_table("Location", |t| t.assert_indexes_count(0))?;
    }

    Ok(())
}

#[test_each_connector]
async fn model_with_multiple_indexes_works(api: &TestApi) -> TestResult {
    let dm = r#"
    model User {
      id         Int       @id
    }

    model Post {
      id        Int       @id
    }

    model Comment {
      id        Int       @id
    }

    model Like {
      id        Int       @id
      user_id   Int
      user      User @relation(fields: [user_id], references: [id])
      post_id   Int
      post      Post @relation(fields: [post_id], references: [id])
      comment_id Int
      comment   Comment @relation(fields: [comment_id], references: [id])

      @@index([post_id])
      @@index([user_id])
      @@index([comment_id])
    }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;
    api.assert_schema()
        .await?
        .assert_table("Like", |table| table.assert_indexes_count(3))?;

    Ok(())
}

#[test_each_connector]
async fn removing_multi_field_unique_index_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id    Int    @id
            field String
            secondField Int

            @@unique([field, secondField])
        }
    "#;

    api.infer_apply(&dm1).send().await?.assert_green()?;

    let result = api.assert_schema().await?.into_schema();

    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == &["field", "secondField"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);

    let dm2 = r#"
            model A {
                id    Int    @id
                field String
                secondField Int
            }
        "#;

    api.infer_apply(&dm2).send().await?.assert_green()?;
    let result = api.assert_schema().await?.into_schema();
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == &["field", "secondField"]);
    assert!(index.is_none());

    Ok(())
}

#[test_each_connector]
async fn index_renaming_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "customName")
            @@index([secondField, field], name: "customNameNonUnique")
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_index_on_columns(&["field", "secondField"], |idx| {
                idx.assert_is_unique()?.assert_name("customName")
            })?
            .assert_index_on_columns(&["secondField", "field"], |idx| {
                idx.assert_is_not_unique()?.assert_name("customNameNonUnique")
            })
    })?;

    let dm2 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "customNameA")
            @@index([secondField, field], name: "customNameNonUniqueA")
        }
    "#;

    let result = api.infer_apply(&dm2).send().await?.into_inner();

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_indexes_count(2)?
            .assert_index_on_columns(&["field", "secondField"], |idx| {
                idx.assert_is_unique()?.assert_name("customNameA")
            })?
            .assert_index_on_columns(&["secondField", "field"], |idx| {
                idx.assert_is_not_unique()?.assert_name("customNameNonUniqueA")
            })
    })?;

    // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
    if !api.is_sqlite() {
        let expected_steps = &["AlterIndex", "AlterIndex"];
        let actual_steps = result.describe_steps();
        assert_eq!(actual_steps, expected_steps);
    }

    Ok(())
}

#[test_each_connector]
async fn index_renaming_must_work_when_renaming_to_default(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customName")
            }
        "#;
    let result = api.infer_and_apply(&dm1).await;
    let index = result
        .sql_schema
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.columns == ["field", "secondField"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);

    let dm2 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField])
            }
        "#;
    let result = api.infer_and_apply(&dm2).await;
    let indexes = result
        .sql_schema
        .table_bang("A")
        .indices
        .iter()
        .filter(|i| i.columns == &["field", "secondField"] && i.name == "A.field_secondField_unique");
    assert_eq!(indexes.count(), 1);

    // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
    if !api.is_sqlite() {
        let expected_steps = &["AlterIndex"];
        let actual_steps = result.migration_output.describe_steps();
        assert_eq!(actual_steps, expected_steps);
    }
}

#[test_each_connector]
async fn index_renaming_must_work_when_renaming_to_custom(api: &TestApi) -> TestResult {
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
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    })?;

    let dm2 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "somethingCustom")
        }
    "#;

    let result = api.infer_apply(&dm2).send().await?.assert_green()?.into_inner();
    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["field", "secondField"], |idx| {
                idx.assert_name("somethingCustom")?.assert_is_unique()
            })
    })?;

    // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
    if !api.is_sqlite() {
        let expected_steps = &["AlterIndex"];
        let actual_steps = result.describe_steps();
        assert_eq!(actual_steps, expected_steps);
    }

    Ok(())
}

#[test_each_connector]
async fn index_updates_with_rename_must_work(api: &TestApi) {
    let dm1 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, secondField], name: "customName")
            }
        "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.name == "customName" && i.columns == &["field", "secondField"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);

    let dm2 = r#"
            model A {
                id Int @id
                field String
                secondField Int

                @@unique([field, id], name: "customNameA")
            }
        "#;
    let result = api.infer_and_apply_forcefully(&dm2).await;
    let indexes = result
        .sql_schema
        .table_bang("A")
        .indices
        .iter()
        .filter(|i| i.columns == &["field", "id"] && i.name == "customNameA");
    assert_eq!(indexes.count(), 1);

    // // Test that we are not dropping and recreating the index. Except in SQLite, because there we are.
    if !api.is_sqlite() {
        let expected_steps = &["DropIndex", "CreateIndex"];
        let actual_steps = result.migration_output.describe_steps();
        assert_eq!(actual_steps, expected_steps);
    }
}

#[test_each_connector]
async fn dropping_a_model_with_a_multi_field_unique_index_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField], name: "customName")
        }
    "#;
    let result = api.infer_and_apply(&dm1).await.sql_schema;
    let index = result
        .table_bang("A")
        .indices
        .iter()
        .find(|i| i.name == "customName" && i.columns == &["field", "secondField"]);
    assert!(index.is_some());
    assert_eq!(index.unwrap().tpe, IndexType::Unique);

    let dm2 = "";
    api.infer_apply(&dm2).send().await?.assert_green()?;

    Ok(())
}

#[test_each_connector(tags("postgres", "mysql"))]
async fn indexes_with_an_automatically_truncated_name_are_idempotent(api: &TestApi) -> TestResult {
    let dm = r#"
        model TestModelWithALongName {
            id Int @id
            looooooooooooongfield String
            evenLongerFieldNameWth String
            omgWhatEvenIsThatLongFieldName String

            @@index([looooooooooooongfield, evenLongerFieldNameWth, omgWhatEvenIsThatLongFieldName])
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("TestModelWithALongName", |table| {
            table.assert_index_on_columns(
                &[
                    "looooooooooooongfield",
                    "evenLongerFieldNameWth",
                    "omgWhatEvenIsThatLongFieldName",
                ],
                |idx| {
                    idx.assert_name(if api.is_mysql() {
                        // The size limit of identifiers is 64 bytes on MySQL
                        // and 63 on Postgres.
                        "TestModelWithALongName.looooooooooooongfield_evenLongerFieldName"
                    } else {
                        "TestModelWithALongName.looooooooooooongfield_evenLongerFieldNam"
                    })
                },
            )
        })?;

    api.schema_push(dm).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

#[test_each_connector]
async fn new_index_with_same_name_as_index_from_dropped_table_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id Int @id
            ownerid String
            owner Owner @relation(fields: [ownerid])

            @@index([ownerid])
        }

        model Other {
            id Int @id
            ownerid String

            @@index([ownerid], name: "ownerid")
        }

        model Owner {
            id String @id
        }
    "#;

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("ownerid", |col| col.assert_is_required())
    })?;

    let dm2 = r#"
        model Owner {
            id Int @id
            ownerid String
            owner Cat @relation(fields: [ownerid])

            @@index([ownerid], name: "ownerid")
        }

        model Cat {
            id String @id
        }
    "#;

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Owner", |table| {
        table.assert_column("ownerid", |col| col.assert_is_required())
    })?;

    Ok(())
}
