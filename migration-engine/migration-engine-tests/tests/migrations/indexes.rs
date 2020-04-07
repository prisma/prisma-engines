use migration_engine_tests::sql::*;

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

            @@index([authorEmail, authorName])
        }
    "#;

    api.infer_apply(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Post", |table| {
        table
            .assert_has_column("authorName")?
            .assert_has_column("authorEmail")?
            .assert_index_on_columns(&["authorEmail", "authorName"], |idx| Ok(idx))
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
            .assert_index_on_columns(&["name", "followersCount"], |idx| idx.assert_is_not_unique())
    })?;

    let dm2 = r#"
        model Test {
            id String @id
            name String
            followersCount Int

            @@unique([name, followersCount], name: "nameAndFollowers")
        }
    "#;

    api.infer_apply(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["name", "followersCount"], |idx| idx.assert_is_unique())
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
