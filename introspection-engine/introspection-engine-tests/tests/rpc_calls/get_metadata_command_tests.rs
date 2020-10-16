use barrel::types;
use introspection_engine_tests::{test_api::*, BarrelMigrationExecutor};
use pretty_assertions::assert_eq;
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector(tags("mysql"))]
async fn metadata_for_mysql_should_work(api: &TestApi) -> crate::TestResult {
    setup(&api.barrel(), api.db_name()).await?;

    let result = api.get_metadata().await?;

    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 49152);

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn metadata_for_postgres_should_work(api: &TestApi) -> crate::TestResult {
    setup(&api.barrel(), api.schema_name()).await?;

    let result = api.get_metadata().await?;

    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 40960);

    Ok(())
}

#[test_each_connector(tags("sqlite"))]
async fn metadata_for_sqlite_should_work(api: &TestApi) -> crate::TestResult {
    setup(&api.barrel(), api.schema_name()).await?;

    let result = api.get_metadata().await?;

    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 16384); // page_size * page_count

    Ok(())
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn metadata_for_mssql_should_work(api: &TestApi) -> crate::TestResult {
    setup(&api.barrel(), api.schema_name()).await?;

    let result = api.get_metadata().await?;

    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 0); // not using anything without writing something first

    Ok(())
}

async fn setup(barrel: &BarrelMigrationExecutor, db_name: &str) -> crate::TestResult {
    barrel
        .execute_with_schema(
            |migration| {
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
            },
            db_name,
        )
        .await?;

    Ok(())
}
