use crate::{custom_assert, get_metadata, introspect, test_each_backend_with_ignores, SqlFamily};
use barrel::types;

pub const SCHEMA_NAME: &str = "introspection-engine";

#[test]
fn introspecting_a_simple_table_with_gql_types_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Sqlite, SqlFamily::Postgres], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("bool", types::boolean());
                t.add_column("float", types::float());
                t.add_column("date", types::date());
                t.add_column("id", types::primary());
                t.add_column("int", types::integer());
                t.add_column("string", types::text());
            });
        });
        let dm = r#"
            model Blog {
                bool    Boolean
                date    DateTime
                float   Float
                id      Int @id
                int     Int 
                string  String
            }
        "#;
        let result = dbg!(get_metadata(test_setup));
        //        custom_assert(&result, dm);
    });
}
