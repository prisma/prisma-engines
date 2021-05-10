use migration_engine_tests::multi_engine_test_api::*;
use sql_schema_describer::DefaultValue;
use test_macros::test_connector;

// MySQL 5.7 and MariaDB are skipped, because the datamodel parser gives us a
// chrono DateTime, and we don't render that in the exact expected format.
#[test_connector(exclude(Mysql57, Mariadb))]
fn datetime_defaults_work(api: TestApi) {
    let engine = api.new_engine();

    let dm = r#"
        model Cat {
            id Int @id
            birthday DateTime @default("2018-01-27T08:00:00Z")
        }
    "#;

    engine.schema_push(dm).send_sync().unwrap().assert_green().unwrap();

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

    engine
        .assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("birthday", |col| col.assert_default(Some(expected_default)))
        })
        .unwrap();
}

#[test_connector(tags(Mariadb, Mysql8), exclude(Vitess))]
fn function_expressions_as_dbgenerated_work(api: TestApi) {
    let engine = api.new_engine();

    let dm = r#"
        model Cat {
            id String @id @default(dbgenerated("(LEFT(UUID(), 8))"))
        }
    "#;

    engine.schema_push(dm).send_sync().unwrap().assert_green().unwrap();

    engine
        .assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("id", |col| {
                col.assert_default(Some(DefaultValue::db_generated("(left(uuid(),8))")))
            })
        })
        .unwrap();
}

#[test_connector(tags(Postgres))]
fn default_dbgenerated_with_type_definitions_should_work(api: TestApi) {
    let engine = api.new_engine();

    let dm = r#"
        model A {
            id String @id @default(dbgenerated("(now())::TEXT"))
        }
    "#;

    engine.schema_push(dm).send_sync().unwrap().assert_green().unwrap();

    engine
        .assert_schema()
        .assert_table("A", |table| {
            table.assert_column("id", |col| {
                col.assert_default(Some(DefaultValue::db_generated("(now())::text")))
            })
        })
        .unwrap();
}

#[test_connector(tags(Postgres))]
fn default_dbgenerated_should_work(api: TestApi) {
    let engine = api.new_engine();

    let dm = r#"
        model A {
            id String @id @default(dbgenerated("(now())"))
        }
    "#;

    engine.schema_push(dm).send_sync().unwrap().assert_green().unwrap();

    engine
        .assert_schema()
        .assert_table("A", |table| {
            table.assert_column("id", |col| {
                col.assert_default(Some(DefaultValue::db_generated("now()")))
            })
        })
        .unwrap();
}

#[test_connector(tags(Postgres))]
fn uuid_default(api: TestApi) {
    let engine = api.new_engine();

    let dm = r#"
        model A {
            id   String @id @db.Uuid
            uuid String @db.Uuid @default("00000000-0000-0000-0016-000000000004")
        }
    "#;

    engine.schema_push(dm).send_sync().unwrap().assert_green().unwrap();

    engine
        .assert_schema()
        .assert_table("A", |table| {
            table.assert_column("uuid", |col| {
                col.assert_default(Some(DefaultValue::value("00000000-0000-0000-0016-000000000004")))
            })
        })
        .unwrap();
}
