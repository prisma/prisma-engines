use crate::{get_metadata, test_backend, BarrelMigrationExecutor};
use quaint::pool::SqlFamily;
use barrel::types;

pub const SCHEMA_NAME: &str = "introspection-engine";

#[test]
fn metadate_for_mysql_should_work() {
    test_backend(SqlFamily::Mysql, |test_setup, barrel| {
        setup(barrel);
        let result = dbg!(get_metadata(test_setup));
        assert_eq!(result.table_count, 3);
        assert_eq!(result.size_in_bytes, 49152);
    });
}

#[test]
fn metadata_for_postgres_should_work() {
    test_backend(SqlFamily::Postgres, |test_setup, barrel| {
        setup(barrel);
        let result = dbg!(get_metadata(test_setup));
        assert_eq!(result.table_count, 3);
        assert_eq!(result.size_in_bytes, 40960);
    });
}

#[test]
fn metadata_for_sqlite_should_work() {
    test_backend(SqlFamily::Sqlite, |test_setup, barrel| {
        setup(barrel);
        let result = dbg!(get_metadata(test_setup));
        assert_eq!(result.table_count, 3);
        assert_eq!(result.size_in_bytes, 0); // page_size * page_count and count is 0
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
