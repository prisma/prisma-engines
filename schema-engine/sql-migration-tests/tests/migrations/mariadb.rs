use quaint::ast as quaint_ast;
use sql_migration_tests::test_api::*;

#[test_connector(tags(Mariadb))]
fn foreign_keys_to_indexes_being_renamed_must_work(api: TestApi) {
    let dm1 = r#"
        model User {
            id String @id
            name String
            posts Post[]

            @@unique([name], name: "idxname", map: "idxname")
        }

        model Post {
            id String @id
            author String
            author_rel User @relation(fields: [author], references: name)
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema()
        .assert_table("User", |table| {
            table.assert_index_on_columns(&["name"], |idx| idx.assert_is_unique().assert_name("idxname"))
        })
        .assert_table("Post", |table| {
            table.assert_fk_on_columns(&["author"], |fk| fk.assert_references("User", &["name"]))
        });

    let insert_post = quaint_ast::Insert::single_into(api.render_table_name("Post"))
        .value("id", "the-post-id")
        .value("author", "steve");

    let insert_user = quaint::ast::Insert::single_into(api.render_table_name("User"))
        .value("id", "the-user-id")
        .value("name", "steve");

    api.query(insert_user.into());
    api.query(insert_post.into());

    let dm2 = r#"
        model User {
            id String @id
            name String
            posts Post[]

            @@unique([name], name: "idxrenamed", map: "idxrenamed")
        }

        model Post {
            id String @id
            author String
            author_rel User @relation(fields: [author], references: name)
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema()
        .assert_table("User", |table| {
            table.assert_index_on_columns(&["name"], |idx| idx.assert_is_unique().assert_name("idxrenamed"))
        })
        .assert_table("Post", |table| {
            table.assert_fk_on_columns(&["author"], |fk| fk.assert_references("User", &["name"]))
        });
}
