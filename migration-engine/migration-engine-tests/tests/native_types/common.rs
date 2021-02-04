use migration_engine_tests::sql::*;

#[test_each_connector]
async fn typescript_starter_schema_is_idempotent_without_native_type_annotations(api: &TestApi) -> TestResult {
    let dm = api.native_types_datamodel(
        r#"
        model Post {
            id        Int     @id @default(autoincrement())
            title     String
            content   String?
            published Boolean @default(false)
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?
        }

        model User {
            id    Int     @id @default(autoincrement())
            email String  @unique
            name  String?
            posts Post[]
        }
    "#,
    );

    api.schema_push(&dm)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;
    api.schema_push(&dm).send().await?.assert_green()?.assert_no_steps()?;
    api.schema_push(&dm).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}
#[test_each_connector]
async fn typescript_starter_schema_starting_without_native_types_is_idempotent(api: &TestApi) -> TestResult {
    let dm = r#"
        model Post {
            id        Int     @id @default(autoincrement())
            title     String
            content   String?
            published Boolean @default(false)
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?
        }

        model User {
            id    Int     @id @default(autoincrement())
            email String  @unique
            name  String?
            posts Post[]
        }
    "#;

    let dm2 = api.native_types_datamodel(dm);

    api.schema_push(dm)
        .send()
        .await?
        .assert_green()?
        .assert_has_executed_steps()?;
    api.schema_push(dm).send().await?.assert_green()?.assert_no_steps()?;
    api.schema_push(&dm2).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

#[test_each_connector(tags("postgres", "mysql", "mssql"))]
async fn bigint_primary_keys_are_idempotent(api: &TestApi) -> TestResult {
    let dm1 = api.native_types_datamodel(
        r#"
            model Cat {
                id BigInt @id @default(autoincrement()) @test_db.BigInt
            }
            "#,
    );

    api.schema_push(&dm1).send().await?.assert_green()?;
    api.schema_push(dm1).send().await?.assert_green()?.assert_no_steps()?;

    let dm2 = api.native_types_datamodel(
        r#"
        model Cat {
            id BigInt @id @default(autoincrement())
        }
        "#,
    );

    api.schema_push(&dm2).send().await?.assert_green()?;
    api.schema_push(dm2).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}
