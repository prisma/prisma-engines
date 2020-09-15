use crate::{test_harness::*, BarrelMigrationExecutor};
use barrel::types;
use pretty_assertions::assert_eq;

#[test_each_connector(tags("sqlite"))]
async fn database_version_for_sqlite_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel).await;
    let result = dbg!(api.get_database_version().await);
    assert_eq!(true, result.contains("3.31"));
}

#[test_each_connector(tags("mysql_5_6"))]
async fn database_version_for_mysql_5_6_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel).await;
    let result = dbg!(api.get_database_version().await);
    assert_eq!(true, result.contains("5.6"));
}

#[test_each_connector(tags("mariadb"))]
async fn database_version_for_mariadb_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel).await;
    let result = dbg!(api.get_database_version().await);
    assert_eq!(true, result.contains("MariaDB"));
}

#[test_each_connector(tags("mysql_8"))]
async fn database_version_for_mysql_8_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel).await;
    let result = dbg!(api.get_database_version().await);
    assert_eq!(true, result.contains("8.0"));
}

#[test_each_connector(tags("postgres"))]
async fn database_version_for_postgres_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel).await;
    let result = dbg!(api.get_database_version().await);
    assert_eq!(true, result.contains("PostgreSQL"));
}

// #[test_each_connector(tags("mssql_2019"))]
// async fn database_version_for_mssql_2019_should_work(api: &TestApi) {
//     let barrel = api.barrel();
//     setup(&barrel).await;
//     let result = dbg!(api.get_database_version().await);
//     assert_eq!(result, "".to_string());
// }

async fn setup(barrel: &BarrelMigrationExecutor) {
    barrel
        .execute(|migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;
}
