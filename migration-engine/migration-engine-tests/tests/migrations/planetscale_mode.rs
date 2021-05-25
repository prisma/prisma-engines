use migration_engine_tests::sync_test_api::*;

#[test_connector(tags(Mysql))]
fn schema_push_planetscale_mode_works(api: TestApi) {
    let dm = format!(
        r#"
        {datasource}

        generator client {{
            provider = "prisma-client-js"
            previewFeatures = ["planetScaleMode"]
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
        datasource = api.datasource_block_with(&[("planetScaleMode", "true")]),
    );

    api.schema_push(&dm).send_sync().assert_green_bang();
    api.schema_push(dm).send_sync().assert_green_bang().assert_no_steps(); // idempotence

    api.assert_schema()
        .assert_table_bang("Post", |table| table.assert_foreign_keys_count(0))
        .assert_table_bang("User", |table| table.assert_foreign_keys_count(0))
        .assert_table_bang("Comment", |table| table.assert_foreign_keys_count(0));
}

#[test_connector]
fn create_migration_planetscale_mode_works(api: TestApi) {
    todo!()
}
