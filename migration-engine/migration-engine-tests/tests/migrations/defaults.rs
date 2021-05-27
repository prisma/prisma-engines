use migration_engine_tests::sync_test_api::*;
use prisma_value::PrismaValue;
use sql_schema_describer::DefaultValue;
use test_macros::test_connector;

// MySQL 5.7 and MariaDB are skipped, because the datamodel parser gives us a
// chrono DateTime, and we don't render that in the exact expected format.
#[test_connector(exclude(Mysql57, Mariadb))]
fn datetime_defaults_work(api: TestApi) {
    let dm = r#"
        model Cat {
            id Int @id
            birthday DateTime @default("2018-01-27T08:00:00Z")
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

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

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("birthday", |col| col.assert_default(Some(expected_default)))
        })
        .unwrap();
}

#[test_connector(tags(Mariadb, Mysql8), exclude(Vitess))]
fn function_expressions_as_dbgenerated_work(api: TestApi) {
    let dm = r#"
        model Cat {
            id String @id @default(dbgenerated("(LEFT(UUID(), 8))"))
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("Cat", |table| {
            table.assert_column("id", |col| {
                col.assert_default(Some(DefaultValue::db_generated("(left(uuid(),8))")))
            })
        })
        .unwrap();
}

#[test_connector(tags(Postgres))]
fn default_dbgenerated_with_type_definitions_should_work(api: TestApi) {
    let dm = r#"
        model A {
            id String @id @default(dbgenerated("(now())::TEXT"))
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("A", |table| {
            table.assert_column("id", |col| {
                col.assert_default(Some(DefaultValue::db_generated("(now())::text")))
            })
        })
        .unwrap();
}

#[test_connector(tags(Postgres))]
fn default_dbgenerated_should_work(api: TestApi) {
    let dm = r#"
        model A {
            id String @id @default(dbgenerated("(now())"))
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("A", |table| {
            table.assert_column("id", |col| {
                col.assert_default(Some(DefaultValue::db_generated("now()")))
            })
        })
        .unwrap();
}

#[test_connector(tags(Postgres))]
fn uuid_default(api: TestApi) {
    let dm = r#"
        model A {
            id   String @id @db.Uuid
            uuid String @db.Uuid @default("00000000-0000-0000-0016-000000000004")
        }
    "#;

    api.schema_push(dm).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("A", |table| {
            table.assert_column("uuid", |col| {
                col.assert_default(Some(DefaultValue::value("00000000-0000-0000-0016-000000000004")))
            })
        })
        .unwrap();
}

#[test_connector]
fn a_default_can_be_dropped(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = r#"
        model User {
            id   Int     @id @default(autoincrement())
            name String  @default("Musti")
        }
    "#;

    api.create_migration("initial", dm1, &directory).send_sync();

    let dm2 = r#"
        model User {
            id   Int     @id @default(autoincrement())
            name String?
        }
    "#;

    api.create_migration("second-migration", dm2, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let output = api.diagnose_migration_history(&directory).send_sync().into_output();

    assert!(output.is_empty());
}

#[test_connector]
fn schemas_with_dbgenerated_work(api: TestApi) {
    let dm1 = r#"
    model User {
        age         Int?
        createdAt   DateTime  @default(dbgenerated())
        email       String?
        firstName   String    @default("")
        id          Int       @id @default(autoincrement())
        lastName    String    @default("")
        password    String?
        updatedAt   DateTime  @default(dbgenerated())
    }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
}

#[test_connector(tags(Mysql8, Mariadb), exclude(Vitess))]
fn schemas_with_dbgenerated_expressions_work(api: TestApi) {
    let dm1 = r#"
    model User {
        int_col Int  @default(dbgenerated("(ABS(8) + ABS(8))"))
        bigint_col BigInt @default(dbgenerated("(ABS(8))"))
        float_col Float @default(dbgenerated("(ABS(8))"))
        decimal_col Decimal @default(dbgenerated("(ABS(8))"))
        boolean_col Boolean @default(dbgenerated("(IFNULL(1,0))"))
        string_col String @default(dbgenerated("(LEFT(UUID(), 8))"))
        dt_col DateTime @default(now())
        dt_col2 DateTime @default(dbgenerated("(SUBDATE(SYSDATE(), 31))"))
        binary_col Bytes @default(dbgenerated("(conv(10,10,2))"))
        enum_col Smolness @default(dbgenerated("(Trim('XSMALL   '))"))
        unsupported_col Unsupported("SET('one', 'two')") @default(dbgenerated("(Trim(' '))"))

        @@ignore
    }

    enum Smolness{
        XSMALL
    }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();
}

#[test_connector]
fn column_defaults_must_be_migrated(api: TestApi) {
    let dm1 = r#"
        model Fruit {
            id Int @id
            name String @default("banana")
        }
    "#;

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("Fruit", |table| {
            table.assert_column("name", |col| {
                col.assert_default(Some(DefaultValue::value(PrismaValue::String("banana".to_string()))))
            })
        })
        .unwrap();

    let dm2 = r#"
        model Fruit {
            id Int @id
            name String @default("mango")
        }
    "#;

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema()
        .assert_table("Fruit", |table| {
            table.assert_column("name", |col| col.assert_default(Some(DefaultValue::value("mango"))))
        })
        .unwrap();
}

#[test_connector]
fn escaped_string_defaults_are_not_arbitrarily_migrated(api: TestApi) {
    use quaint::ast::Insert;

    let dm1 = r#"
        model Fruit {
            id String @id @default(cuid())
            seasonality String @default("\"summer\"")
            contains String @default("'potassium'")
            sideNames String @default("top\ndown")
            size Float @default(12.3)
        }
    "#;

    api.schema_push(dm1)
        .migration_id(Some("first migration"))
        .send_sync()
        .assert_green()
        .unwrap();

    let insert = Insert::single_into(api.render_table_name("Fruit"))
        .value("id", "apple-id")
        .value("sideNames", "stem and the other one")
        .value("contains", "'vitamin C'")
        .value("seasonality", "september");

    api.query(insert.into());

    api.schema_push(dm1)
        .migration_id(Some("second migration"))
        .send_sync()
        .assert_green()
        .unwrap()
        .assert_no_steps();

    let sql_schema = api.assert_schema().into_schema();
    let table = sql_schema.table_bang(&api.normalize_identifier("Fruit"));

    if api.is_mssql() {
        let default = table.column("sideNames").and_then(|c| c.default.clone()).unwrap();
        assert_eq!(DefaultValue::value("top\ndown").kind(), default.kind());
        assert!(default
            .constraint_name()
            .map(|cn| cn.starts_with("DF__Fruit__sideNames"))
            .unwrap());

        let default = table.column("contains").and_then(|c| c.default.clone()).unwrap();
        assert_eq!(DefaultValue::value("'potassium'").kind(), default.kind());
        assert!(default
            .constraint_name()
            .map(|cn| cn.starts_with("DF__Fruit__contains"))
            .unwrap());

        let default = table.column("seasonality").and_then(|c| c.default.clone()).unwrap();
        assert_eq!(DefaultValue::value(r#""summer""#).kind(), default.kind());
        assert!(default
            .constraint_name()
            .map(|cn| cn.starts_with("DF__Fruit__seasonali"))
            .unwrap());
    } else {
        assert_eq!(
            table.column("sideNames").and_then(|c| c.default.clone()),
            Some(DefaultValue::value(PrismaValue::String("top\ndown".to_string())))
        );

        assert_eq!(
            table.column("contains").and_then(|c| c.default.clone()),
            Some(DefaultValue::value(PrismaValue::String("'potassium'".to_string())))
        );

        assert_eq!(
            table.column("seasonality").and_then(|c| c.default.clone()),
            Some(DefaultValue::value(PrismaValue::String(r#""summer""#.to_string())))
        );
    }
}
