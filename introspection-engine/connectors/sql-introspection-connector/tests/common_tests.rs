mod test_harness;

use barrel::types;
use test_harness::*;

pub const SCHEMA_NAME: &str = "introspection-engine";

#[test]
fn introspecting_a_simple_table_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Postgres, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        });
        let dm = r#"
            model Blog {
                id Int @id
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(result, dm.to_string());
    });
}

#[test]
fn introspecting_a_table_with_compound_primary_keys_must_work() {
    test_each_backend_with_ignores(vec![SqlFamily::Postgres, SqlFamily::Mysql], |test_setup, barrel| {
        let _setup_schema = barrel.execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        });
        let dm = r#"
            model Blog {
                id Int @id
            }
        "#;
        let result = dbg!(introspect(test_setup));
        custom_assert(result, dm.to_string());
    });
}
