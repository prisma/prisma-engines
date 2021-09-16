use migration_engine_tests::test_api::*;

#[test_connector(preview_features("referentialIntegrity"))]
fn typescript_starter_schema_is_idempotent_without_native_type_annotations(api: TestApi) {
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

    api.schema_push_w_datasource(dm)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(exclude(Mssql), preview_features("referentialIntegrity"))]
// TODO (matthias) changing towards having a provider specified in the middle of the test messes with some weird hard-coded
// Does this test even make sense? When using the migrate CLI you cannot NOT have a provider specified. This only works in our weird
// test setup
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

    api.schema_push_w_datasource(dm)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Postgres, Mysql, Mssql))]
fn bigint_primary_keys_are_idempotent(api: TestApi) {
    let dm1 = r#"
            model Cat {
                id BigInt @id @default(autoincrement()) @db.BigInt
            }
        "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.schema_push_w_datasource(dm1)
        .send()
        .assert_green()
        .assert_no_steps();

    let dm2 = r#"
        model Cat {
            id BigInt @id @default(autoincrement())
        }
        "#;

    api.schema_push_w_datasource(dm2).send().assert_green();
    api.schema_push_w_datasource(dm2)
        .send()
        .assert_green()
        .assert_no_steps();
}
