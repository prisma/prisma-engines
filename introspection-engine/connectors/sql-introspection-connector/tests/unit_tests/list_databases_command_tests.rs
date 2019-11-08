use crate::{list_databases, test_backend, BarrelMigrationExecutor, SqlFamily};
use barrel::types;

pub const SCHEMA_NAME: &str = "introspection-engine";

#[test]
fn databases_for_mysql_should_work() {
    test_backend(SqlFamily::Mysql, |test_setup, barrel| {
        setup(barrel);
        let result = dbg!(list_databases(test_setup));
        let vec: Vec<String> = Vec::new();
        assert_eq!(result, vec);
    });
}

#[test]
fn databases_for_postgres_should_work() {
    test_backend(SqlFamily::Postgres, |test_setup, barrel| {
        setup(barrel);
        let result = dbg!(list_databases(test_setup));
        let vec: Vec<String> = Vec::new();
        assert_eq!(result, vec);
    });
}

#[test]
fn databases_for_sqlite_should_work() {
    test_backend(SqlFamily::Sqlite, |test_setup, barrel| {
        setup(barrel);
        let result = dbg!(list_databases(test_setup));
        let vec: Vec<String> = Vec::new();
        assert_eq!(result, vec);
    });
}

fn setup(barrel: &BarrelMigrationExecutor) {
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("Blog", |t| {
            t.add_column("bool", types::boolean());
            t.add_column("float", types::float());
            t.add_column("date", types::date());
            t.add_column("id", types::primary());
            t.add_column("int", types::integer());
            t.add_column("string", types::text());
        });

        migration.create_table("Blog2", |t| {
            t.add_column("id", types::primary());
            t.add_column("int", types::integer());
            t.add_column("string", types::text());
        });

        migration.create_table("Blog3", |t| {
            t.add_column("bool", types::boolean());
            t.add_column("float", types::float());
            t.add_column("date", types::date());
            t.add_column("id", types::primary());
        });
    });
}
