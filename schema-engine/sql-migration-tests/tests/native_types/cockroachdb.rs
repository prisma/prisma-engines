use sql_migration_tests::test_api::*;

#[test_connector(tags(CockroachDb))]
fn typescript_starter_schema_is_idempotent_without_native_type_annotations(api: TestApi) {
    let dm = r#"
        model Post {
            id        BigInt     @id @default(autoincrement())
            title     String
            content   String?
            published Boolean @default(false)
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  BigInt?
        }

        model User {
            id    BigInt     @id @default(autoincrement())
            email String  @unique
            name  String?
            posts Post[]
            age   Int
        }
    "#;

    api.schema_push_w_datasource(dm)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}
