mod test_harness;

use barrel::{Migration, types};
use test_harness::*;
use std::sync::Arc;



#[test]
fn adding_a_model_for_an_existing_table_must_work() {
    test_each_backend(|test_setup, api, barrel| {
        let initial_result = barrel.execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        });
        let dm = r#"
            model Blog {
                id Int @id
            }
        "#;
//        let result = introspect(test_setup, api);
//        assert_eq!(result, dm);
    });
}



struct BarrelMigrationExecutor {
//    database: Arc<dyn MigrationDatabase + Send + Sync>,
    sql_variant: barrel::backend::SqlVariant,
}

impl BarrelMigrationExecutor {
    fn execute<F>(&self, mut migrationFn: F)
        where
            F: FnMut(&mut Migration) -> (),
    {
        let mut migration = Migration::new().schema("a");
        migrationFn(&mut migration);
        let full_sql = dbg!(migration.make_from(self.sql_variant));
        run_full_sql(&self.database, &full_sql);
    }
}

fn run_full_sql(database: &Arc<dyn MigrationDatabase + Send + Sync>, full_sql: &str) {
    for sql in full_sql.split(";") {
        if sql != "" {
            database.query_raw(SCHEMA_NAME, &sql, &[]).unwrap();
        }
    }
}

fn test_each_backend<F>(test_fn: F)
    where
        F: Fn(&TestSetup, &dyn GenericApi, &BarrelMigrationExecutor) -> () + std::panic::RefUnwindSafe,
{
    test_each_backend_with_ignores(Vec::new(), test_fn);
}

fn test_each_backend_with_ignores<F>(ignores: Vec<SqlFamily>, test_fn: F)
    where
        F: Fn(&TestSetup, &dyn GenericApi, &BarrelMigrationExecutor) -> () + std::panic::RefUnwindSafe,
{
    // SQLite
//    if !ignores.contains(&SqlFamily::Sqlite) {
//        println!("Testing with SQLite now");
//        let (inspector, test_setup) = get_sqlite();
//
//        println!("Running the test function now");
//        let connector = SqlMigrationConnector::sqlite(&sqlite_test_file()).unwrap();
//        let api = test_api(connector);
//
//        let barrel_migration_executor = BarrelMigrationExecutor {
//            inspector,
//            database: Arc::clone(&test_setup.database),
//            sql_variant: SqlVariant::Sqlite,
//        };
//
//        test_fn(&test_setup, &api, &barrel_migration_executor);
//    } else {
//        println!("Ignoring SQLite")
//    }
//    // POSTGRES
//    if !ignores.contains(&SqlFamily::Postgres) {
//        println!("Testing with Postgres now");
//        let (inspector, test_setup) = get_postgres();
//
//        println!("Running the test function now");
//        let connector = SqlMigrationConnector::postgres(&postgres_url(), false).unwrap();
//        let api = test_api(connector);
//
//        let barrel_migration_executor = BarrelMigrationExecutor {
//            inspector,
//            database: Arc::clone(&test_setup.database),
//            sql_variant: SqlVariant::Pg,
//        };
//
//        test_fn(&test_setup, &api, &barrel_migration_executor);
//    } else {
//        println!("Ignoring Postgres")
//    }
}
