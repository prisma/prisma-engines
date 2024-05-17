use quaint::prelude::Insert;
use schema_core::json_rpc::types::SchemasContainer;
use sql_migration_tests::test_api::*;

#[test_connector(tags(Sqlite))]
fn sqlite_must_recreate_indexes(api: TestApi) {
    // SQLite must go through a complicated migration procedure which requires dropping and recreating indexes. This test checks that.
    // We run them still against each connector.
    let dm1 = r#"
        model A {
            id Int @id
            field String @unique
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id    Int    @id
            field String @unique
            other String
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["field"], |idx| idx.assert_is_unique())
    });
}

#[test_connector(tags(Sqlite))]
fn sqlite_must_recreate_multi_field_indexes(api: TestApi) {
    // SQLite must go through a complicated migration procedure which requires dropping and recreating indexes. This test checks that.
    // We run them still against each connector.
    let dm1 = r#"
        model A {
            id Int @id
            field String
            secondField Int

            @@unique([field, secondField])
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    });

    let dm2 = r#"
        model A {
            id    Int    @id
            field String
            secondField Int
            other String

            @@unique([field, secondField])
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_index_on_columns(&["field", "secondField"], |idx| idx.assert_is_unique())
    });
}

// This is necessary because of how INTEGER PRIMARY KEY works on SQLite. This has already caused problems.
#[test_connector(tags(Sqlite))]
fn creating_a_model_with_a_non_autoincrement_id_column_is_idempotent(api: TestApi) {
    let dm = r#"
        model Cat {
            id  Int @id
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Sqlite))]
fn treat_nullable_integer_primary_key_as_required(api: TestApi) {
    let schema = r#"CREATE TABLE "a" ("id" INTEGER NULL, PRIMARY KEY("id"));"#;
    api.raw_cmd(schema);

    let dm = r#"
        model a {
          id Int @id @default(autoincrement())
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Sqlite))]
fn bigint_defaults_work(api: TestApi) {
    let schema = r#"
        datasource mypg {
            provider = "sqlite"
            url = env("TEST_DATABASE_URL")
        }

        model foo {
          id  String @id
          bar BigInt @default(0)
        }
    "#;
    let sql = expect![[r#"
        -- CreateTable
        CREATE TABLE "foo" (
            "id" TEXT NOT NULL PRIMARY KEY,
            "bar" BIGINT NOT NULL DEFAULT 0
        );
    "#]];
    api.expect_sql_for_schema(schema, &sql);

    api.schema_push(schema).send().assert_green();
    api.schema_push(schema).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Sqlite))]
fn default_string_with_escaped_unicode(api: TestApi) {
    let dm = r#"
        datasource mypg {
            provider = "sqlite"
            url = env("TEST_DATABASE_URL")
        }

        model test {
            name String @id @default("\uFA44\ufa44")
        }
    "#;

    let expected = expect![[r#"
        -- CreateTable
        CREATE TABLE "test" (
            "name" TEXT NOT NULL PRIMARY KEY DEFAULT '梅梅'
        );
    "#]];

    api.expect_sql_for_schema(dm, &expected);

    api.schema_push(dm).send().assert_green().assert_has_executed_steps();
    api.schema_push(dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Sqlite))]
fn unique_constraint_errors_in_migrations(api: TestApi) {
    let dm = r#"
        model Fruit {
            id   Int @id @default(autoincrement())
            name String
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let insert = Insert::multi_into(api.render_table_name("Fruit"), ["name"])
        .values(("banana",))
        .values(("apple",))
        .values(("banana",));

    api.query(insert.into());

    let dm2 = r#"
        model Fruit {
            id   Int @id @default(autoincrement())
            name String @unique
        }
    "#;

    let res = api
        .schema_push_w_datasource(dm2)
        .force(true)
        .migration_id(Some("the-migration"))
        .send_unwrap_err()
        .to_user_facing();

    assert!(serde_json::to_string_pretty(&res)
        .unwrap()
        .contains("UNIQUE constraint failed: Fruit.name"));
}

#[test]
fn introspecting_a_non_existing_db_fails() {
    test_setup::only!(Sqlite);

    let dm = r#"
        datasource db {
            provider = "sqlite"
            url = "file:/tmp/definitelies-does-not-exist.sqlite"
        }
    "#;

    let api = schema_core::schema_api(None, None).unwrap();
    let err = tok(api.introspect(schema_core::json_rpc::types::IntrospectParams {
        composite_type_depth: -1,
        force: false,
        schema: SchemasContainer {
            files: vec![SchemaContainer {
                path: "schema.prisma".to_string(),
                content: dm.to_string(),
            }],
        },
        namespaces: None,
    }))
    .unwrap_err();

    let expected = expect![[r#"
        Database `definitelies-does-not-exist.sqlite` does not exist at `/tmp/definitelies-does-not-exist.sqlite`.
    "#]];
    expected.assert_eq(&err.to_string());
}
