use migration_engine_tests::*;
use quaint::ast as quaint_ast;

#[test_each_connector(tags("mariadb"))]
async fn foreign_keys_to_indexes_being_renamed_must_work(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model User {
            id String @id
            name String

            @@unique([name], name: "idxname")
        }

        model Post {
            id String @id
            author String
            author_rel User @relation(fields: [author], references: name)
        }
    "#;

    api.infer_apply(dm1).send_assert().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("User", |table| {
            table.assert_index_on_columns(&["name"], |idx| idx.assert_name("idxname"))
        })?
        .assert_table("Post", |table| {
            table.assert_fk_on_columns(&["author"], |fk| fk.assert_references("User", &["name"]))
        })?;

    let insert_post = quaint_ast::Insert::single_into(api.render_table_name("Post"))
        .value("id", "the-post-id")
        .value("author", "steve");

    let insert_user = quaint::ast::Insert::single_into(api.render_table_name("User"))
        .value("id", "the-user-id")
        .value("name", "steve");

    let db = api.database();
    db.query(insert_user.into()).await?;
    db.query(insert_post.into()).await?;

    let dm2 = r#"
        model User {
            id String @id
            name String

            @@unique([name], name: "idxrenamed")
        }

        model Post {
            id String @id
            author String
            author_rel User @relation(fields: [author], references: name)
        }
    "#;

    api.infer_apply(dm2).send_assert().await?.assert_green()?;

    api.assert_schema()
        .await?
        .assert_table("User", |table| {
            table.assert_index_on_columns(&["name"], |idx| idx.assert_name("idxrenamed"))
        })?
        .assert_table("Post", |table| {
            table.assert_fk_on_columns(&["author"], |fk| fk.assert_references("User", &["name"]))
        })?;

    Ok(())
}
