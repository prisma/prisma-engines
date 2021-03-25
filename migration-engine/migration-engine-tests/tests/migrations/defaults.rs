use migration_engine_tests::{multi_engine_test_api::*, TestResult};
use sql_schema_describer::DefaultValue;
use test_macros::test_connectors;

// MySQL 5.7 and MariaDB are skipped, because the datamodel parser gives us a
// chrono DateTime, and we don't render that in the exact expected format.
#[test_connectors(ignore("mysql_5_7", "mariadb"))]
async fn datetime_defaults_work(api: TestApi) -> TestResult {
    api.initialize().await?;

    let engine = api.new_engine().await?;

    let dm = r#"
        model Cat {
            id Int @id
            birthday DateTime @default("2018-01-27T08:00:00Z")
        }
    "#;

    engine.schema_push(dm).send().await?.assert_green()?;

    let expected_default = if api.is_postgres() {
        DefaultValue::db_generated("'2018-01-27 08:00:00'::timestamp without time zone")
    } else if api.is_mssql() {
        DefaultValue::db_generated("2018-01-27 08:00:00 +00:00")
    } else if api.is_mysql_mariadb() {
        DefaultValue::db_generated("2018-01-27T08:00:00+00:00")
    } else if api.is_mysql_8() || api.is_mysql_5_6() {
        DefaultValue::db_generated("2018-01-27 08:00:00.000")
    } else {
        DefaultValue::db_generated("'2018-01-27 08:00:00 +00:00'")
    };

    engine.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("birthday", |col| col.assert_default(Some(expected_default)))
    })?;

    Ok(())
}

#[test_connectors(tags("mariadb", "mysql_8"))]
async fn function_expressions_as_dbgenerated_work(api: TestApi) -> TestResult {
    api.initialize().await?;

    let engine = api.new_engine().await?;

    let dm = r#"
        model Cat {
            id String @id @default(dbgenerated("(LEFT(UUID(), 8))"))
        }
    "#;

    engine.schema_push(dm).send().await?.assert_green()?;

    engine.assert_schema().await?.assert_table("Cat", |table| {
        table.assert_column("id", |col| {
            col.assert_default(Some(DefaultValue::db_generated("(left(uuid(),8))")))
        })
    })?;

    Ok(())
}
