use migration_engine_tests::multi_engine_test_api::*;
use quaint::ast as quaint_ast;

#[test_connector(tags(Mariadb))]
fn foreign_keys_to_indexes_being_renamed_must_work(api: TestApi) {
    let engine = api.new_engine();
    let dm1 = r#"
        model User {
            id String @id
            name String
            posts Post[]

            @@unique([name], name: "idxname")
        }

        model Post {
            id String @id
            author String
            author_rel User @relation(fields: [author], references: name)
        }
    "#;

    engine.schema_push(dm1).send_sync().unwrap().assert_green().unwrap();

    engine
        .assert_schema()
        .assert_table("User", |table| {
            table.assert_index_on_columns(&["name"], |idx| idx.assert_name("idxname"))
        })
        .unwrap()
        .assert_table("Post", |table| {
            table.assert_fk_on_columns(&["author"], |fk| fk.assert_references("User", &["name"]))
        })
        .unwrap();

    let insert_post = quaint_ast::Insert::single_into(engine.render_table_name("Post"))
        .value("id", "the-post-id")
        .value("author", "steve");

    let insert_user = quaint::ast::Insert::single_into(engine.render_table_name("User"))
        .value("id", "the-user-id")
        .value("name", "steve");

    engine.query(insert_user.into());
    engine.query(insert_post.into());

    let dm2 = r#"
        model User {
            id String @id
            name String
            posts Post[]

            @@unique([name], name: "idxrenamed")
        }

        model Post {
            id String @id
            author String
            author_rel User @relation(fields: [author], references: name)
        }
    "#;

    engine.schema_push(dm2).send_sync().unwrap().assert_green().unwrap();

    engine
        .assert_schema()
        .assert_table("User", |table| {
            table.assert_index_on_columns(&["name"], |idx| idx.assert_name("idxrenamed"))
        })
        .unwrap()
        .assert_table("Post", |table| {
            table.assert_fk_on_columns(&["author"], |fk| fk.assert_references("User", &["name"]))
        })
        .unwrap();
}
