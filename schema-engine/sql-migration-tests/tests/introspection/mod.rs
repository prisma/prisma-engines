use expect_test::expect;
use indoc::indoc;
use quaint::connector::rusqlite;
use schema_core::{
    DatasourceUrls,
    json_rpc::types::{IntrospectParams, SchemasContainer},
};
use sql_migration_tests::test_api::SchemaContainer;
use test_setup::runtime::run_with_thread_local_runtime as tok;

#[test]
fn introspect_force_with_invalid_schema() {
    test_setup::only!(Sqlite);

    let db_path = test_setup::sqlite_test_url("introspect_force_with_invalid_schema");

    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE corgis (bites BOOLEAN)").unwrap();
    }

    let schema = indoc!(
        r#"
        datasource sqlitedb {
            provider = "sqlite"
        }

        model This_Is_Blatantly_Not_Valid_and_An_Outrage {
            pk Bytes @unknownAttributeThisIsNotValid
        }
    "#
    );

    let api =
        schema_core::schema_api_without_extensions(Some(schema.to_owned()), DatasourceUrls::from_url(db_path), None)
            .unwrap();

    let params = IntrospectParams {
        schema: SchemasContainer {
            files: vec![SchemaContainer {
                path: "schema.prisma".to_string(),
                content: schema.to_owned(),
            }],
        },
        base_directory_path: "/".to_string(),
        force: true,
        composite_type_depth: 0,
        namespaces: None,
    };

    let result = tok(api.introspect(params)).unwrap();
    let result = result.schema.files.first().map(|dm| dm.content.as_str()).unwrap();

    let expected = expect![[r#"
        datasource sqlitedb {
          provider = "sqlite"
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.
        model corgis {
          bites Boolean?

          @@ignore
        }
    "#]];

    expected.assert_eq(result);
}

#[test]
fn introspect_no_force_with_invalid_schema() {
    test_setup::only!(Sqlite);

    let db_path = test_setup::sqlite_test_url("introspect_no_force_with_invalid_schema");

    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE corgis (bites BOOLEAN)").unwrap();
    }

    let schema = indoc::indoc!(
        r#"
        datasource sqlitedb {
          provider = "sqlite"
        }

        model This_Is_Blatantly_Not_Valid_and_An_Outrage {
          pk Bytes @unknownAttributeThisIsNotValid
        }
    "#
    );

    let api =
        schema_core::schema_api_without_extensions(Some(schema.to_owned()), DatasourceUrls::from_url(db_path), None)
            .unwrap();

    let params = IntrospectParams {
        schema: SchemasContainer {
            files: vec![SchemaContainer {
                path: "schema.prisma".to_string(),
                content: schema.to_owned(),
            }],
        },
        base_directory_path: "/".to_string(),
        force: false,
        composite_type_depth: 0,
        namespaces: None,
    };

    let ufe = tok(api.introspect(params)).unwrap_err().to_user_facing();

    let expected = expect![[r#"
        [1;91merror[0m: [1mAttribute not known: "@unknownAttributeThisIsNotValid".[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0mmodel This_Is_Blatantly_Not_Valid_and_An_Outrage {
        [1;94m 7 | [0m  pk Bytes [1;91m@unknownAttributeThisIsNotValid[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(ufe.message());
}
