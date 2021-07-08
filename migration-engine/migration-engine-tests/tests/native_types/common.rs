use migration_engine_tests::sync_test_api::*;

#[test_connector]
fn typescript_starter_schema_is_idempotent_without_native_type_annotations(api: TestApi) {
    let dm = api.datamodel_with_provider(
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
        .assert_green_bang()
        .assert_has_executed_steps();
    api.schema_push(&dm).send().assert_green_bang().assert_no_steps();
    api.schema_push(&dm).send().assert_green_bang().assert_no_steps();
}
#[test_connector]
fn typescript_starter_schema_starting_without_native_types_is_idempotent(api: TestApi) {
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

    let dm2 = format!("{}\n{}", api.datasource_block(), dm);

    api.schema_push(dm)
        .send()
        .assert_green_bang()
        .assert_has_executed_steps();
    api.schema_push(dm).send().assert_green_bang().assert_no_steps();
    api.schema_push(&dm2).send().assert_green_bang().assert_no_steps();
}

#[test_connector(tags(Postgres, Mysql, Mssql))]
fn bigint_primary_keys_are_idempotent(api: TestApi) {
    let dm1 = api.datamodel_with_provider(
        r#"
            model Cat {
                id BigInt @id @default(autoincrement()) @db.BigInt
            }
        "#,
    );

    api.schema_push(&dm1).send().assert_green_bang();
    api.schema_push(dm1).send().assert_green_bang().assert_no_steps();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id BigInt @id @default(autoincrement())
        }
        "#,
    );

    api.schema_push(&dm2).send().assert_green_bang();
    api.schema_push(dm2).send().assert_green_bang().assert_no_steps();
}
