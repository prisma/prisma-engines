use migration_engine_tests::test_api::*;

#[test_connector]
fn schema_push_referential_integrity_prisma_works(api: TestApi) {
    let dm = format!(
        r#"
        {datasource}

        generator client {{
            provider = "prisma-client-js"
            previewFeatures = ["referentialIntegrity"]
        }}

        model Post {{
            id          String  @id
            authorId    Int?
            author      User? @relation(fields: [authorId], references: [id])
            comments    Comment[]
        }}

        model User {{
            id          Int @id
            posts       Post[]
            comments    Comment[]
        }}

        model Comment {{
            id Int @id
            authorId    Int
            author      User @relation(fields: [authorId], references: [id])
            postId      String
            post        Post @relation(fields: [postId], references: [id])
        }}
        "#,
        datasource = api.datasource_block_with(&[("referentialIntegrity", "\"prisma\"")]),
    );

    api.schema_push(&dm).send().assert_green();
    api.schema_push(dm).send().assert_green().assert_no_steps(); // idempotence

    api.assert_schema()
        .assert_table("Post", |table| table.assert_foreign_keys_count(0))
        .assert_table("User", |table| table.assert_foreign_keys_count(0))
        .assert_table("Comment", |table| table.assert_foreign_keys_count(0));
}

#[test_connector]
fn create_migration_referential_integrity_prisma_works(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm = format!(
        r#"
        {datasource}

        generator client {{
            provider = "prisma-client-js"
            previewFeatures = ["referentialIntegrity"]
        }}

        model Post {{
            id          String  @id
            authorId    Int?
            author      User? @relation(fields: [authorId], references: [id])
            comments    Comment[]
        }}

        model User {{
            id          Int @id
            posts       Post[]
            comments    Comment[]
        }}

        model Comment {{
            id Int @id
            authorId    Int
            author      User @relation(fields: [authorId], references: [id])
            postId      String
            post        Post @relation(fields: [postId], references: [id])
        }}
        "#,
        datasource = api.datasource_block_with(&[("referentialIntegrity", "\"prisma\"")]),
    );

    api.create_migration("01init", &dm, &migrations_directory)
        .send_sync()
        .assert_migration_directories_count(1);

    // Check that the migration is idempotent
    api.create_migration("02second", &dm, &migrations_directory)
        .send_sync()
        .assert_migration_directories_count(1);

    // Check that the migration applies
    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["01init"]);

    // Check that no drift is detected
    let diagnostic = api
        .diagnose_migration_history(&migrations_directory)
        .send_sync()
        .into_output();

    assert!(diagnostic.drift.is_none());

    api.assert_schema()
        .assert_table("Post", |table| table.assert_foreign_keys_count(0))
        .assert_table("User", |table| table.assert_foreign_keys_count(0))
        .assert_table("Comment", |table| table.assert_foreign_keys_count(0));
}
