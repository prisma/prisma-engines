use expect_test::expect;
use quaint::connector::rusqlite;
use test_setup::runtime::run_with_tokio as tok;

#[test]
fn introspect_force_with_invalid_schema() {
    test_setup::only!(Sqlite);

    let db_path = test_setup::sqlite_test_url("introspect_force_with_invalid_schema");

    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE corgis (bites BOOLEAN)").unwrap();
    }

    let schema = format!(
        r#"
        datasource sqlitedb {{
            provider = "sqlite"
            url = "{db_path}"
        }}

        model This_Is_Blatantly_Not_Valid_and_An_Outrage {{
            pk Bytes @unknownAttributeThisIsNotValid
        }}
    "#
    );

    let result = &tok(introspection_core::RpcImpl::introspect_internal(
        schema,
        true,
        Default::default(),
    ))
    .unwrap()
    .datamodel
    .replace(db_path.as_str(), "<db_path>");

    let expected = expect![[r#"
        datasource sqlitedb {
          provider = "sqlite"
          url      = "<db_path>"
        }

        /// The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.
        model corgis {
          bites Boolean?

          @@ignore
        }
    "#]];

    expected.assert_eq(result);
}
