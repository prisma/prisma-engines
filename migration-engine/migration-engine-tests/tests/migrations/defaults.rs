use migration_engine_tests::test_api::*;
use prisma_value::PrismaValue;
use sql_schema_describer::{DefaultKind, DefaultValue};

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

    api.schema_push_w_datasource(dm).send().assert_green();

    let expected_default = if api.is_cockroach() {
        DefaultValue::db_generated("'2018-01-27 08:00:00'::TIMESTAMP")
    } else if api.is_postgres() {
        DefaultValue::db_generated("'2018-01-27 08:00:00'::timestamp without time zone")
    } else if api.is_mssql() {
        let mut df = DefaultValue::db_generated("2018-01-27 08:00:00 +00:00");
        df.set_constraint_name("Cat_birthday_df");
        df
    } else if api.is_mysql_mariadb() {
        DefaultValue::db_generated("'2018-01-27T08:00:00+00:00'")
    } else if api.is_mysql_8() || api.is_mysql_5_6() {
        DefaultValue::db_generated("'2018-01-27 08:00:00.000'")
    } else {
        DefaultValue::db_generated("'2018-01-27 08:00:00 +00:00'")
    };

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("birthday", |col| col.assert_default(Some(expected_default)))
    });

    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mysql8), exclude(Vitess))]
fn binary_dbgenerated_defaults_should_work(api: TestApi) {
    // https://github.com/prisma/prisma/issues/10715

    let schema = r#"
        model A {
          id Bytes @id @default(dbgenerated("(uuid_to_bin(uuid()))")) @db.Binary(16)
        }
    "#;

    api.schema_push_w_datasource(schema).send().assert_green();

    let def_val = DefaultValue::db_generated("(uuid_to_bin(uuid()))");

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("id", |col| col.assert_default(Some(def_val)))
    });

    api.schema_push_w_datasource(schema).send().assert_no_steps();
}

#[test_connector(tags(Mysql), exclude(Mariadb))]
fn datetime_dbgenerated_defaults(api: TestApi) {
    let dm = indoc::indoc! {r#"
        model A {
          id Int       @id @default(autoincrement())
          d1 DateTime? @default(dbgenerated("'2020-01-01'")) @db.Date
          d2 DateTime? @default(dbgenerated("'2038-01-19 03:14:08'")) @db.DateTime(0)
          d3 DateTime? @default(dbgenerated("'16:20:00'")) @db.Time(0)
        }
    "#};

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("d1", |col| {
            col.assert_default(Some(DefaultValue::db_generated("'2020-01-01'")))
        });

        table.assert_column("d2", |col| {
            col.assert_default(Some(DefaultValue::db_generated("'2038-01-19 03:14:08'")))
        });

        table.assert_column("d3", |col| {
            col.assert_default(Some(DefaultValue::db_generated("'16:20:00'")))
        })
    });

    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mariadb))]
fn datetime_dbgenerated_defaults_mariadb(api: TestApi) {
    let dm = indoc::indoc! {r#"
        model A {
          id Int       @id @default(autoincrement())
          d1 DateTime? @default(dbgenerated("('2020-01-01')")) @db.Date
          d2 DateTime? @default(dbgenerated("('2038-01-19 03:14:08')")) @db.DateTime(0)
          d3 DateTime? @default(dbgenerated("('16:20:00')")) @db.Time(0)
        }
    "#};

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("d1", |col| {
            col.assert_default(Some(DefaultValue::db_generated("('2020-01-01')")))
        });

        table.assert_column("d2", |col| {
            col.assert_default(Some(DefaultValue::db_generated("('2038-01-19 03:14:08')")))
        });

        table.assert_column("d3", |col| {
            col.assert_default(Some(DefaultValue::db_generated("('16:20:00')")))
        })
    });

    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Mariadb, Mysql8), exclude(Vitess))]
fn function_expressions_as_dbgenerated_work(api: TestApi) {
    let dm = r#"
        model Cat {
            id String @id @default(dbgenerated("(LEFT(UUID(), 8))"))
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Cat", |table| {
        table.assert_column("id", |col| {
            col.assert_default(Some(DefaultValue::db_generated("(left(uuid(),8))")))
        })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn default_dbgenerated_with_type_definitions_should_work(api: TestApi) {
    let dm = r#"
        model A {
            id String @id @default(dbgenerated("(now())::TEXT"))
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("id", |col| {
            col.assert_default(Some(DefaultValue::db_generated("(now())::text")))
        })
    });
}

#[test_connector(tags(CockroachDb))]
fn default_dbgenerated_with_type_definitions_should_work_cockroach(api: TestApi) {
    let dm = r#"
        model A {
            id String @id @default(dbgenerated("(now()::text)"))
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("id", |col| {
            col.assert_default(Some(DefaultValue::db_generated("now()::STRING")))
        })
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn default_dbgenerated_should_work(api: TestApi) {
    let dm = r#"
        model A {
            id String @id @default(dbgenerated("(now())"))
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("id", |col| {
            col.assert_default(Some(DefaultValue::db_generated("now()")))
        })
    });
}

#[test_connector(tags(CockroachDb))]
fn default_dbgenerated_should_work_cockroach(api: TestApi) {
    let dm = r#"
        model A {
            id DateTime @id @default(dbgenerated("(now())"))
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("id", |col| col.assert_default(Some(DefaultValue::now())))
    });
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn uuid_default(api: TestApi) {
    let dm = r#"
        model A {
            id   String @id @db.Uuid
            uuid String @db.Uuid @default("00000000-0000-0000-0016-000000000004")
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("uuid", |col| {
            col.assert_default(Some(DefaultValue::db_generated(
                "'00000000-0000-0000-0016-000000000004'::uuid",
            )))
        })
    });
}

#[test_connector(tags(CockroachDb))]
fn uuid_default_cockroach(api: TestApi) {
    let dm = r#"
        model A {
            id   String @id @db.Uuid
            uuid String @db.Uuid @default("00000000-0000-0000-0016-000000000004")
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("uuid", |col| {
            col.assert_default(Some(DefaultValue::db_generated(
                "'00000000-0000-0000-0016-000000000004'::UUID",
            )))
        })
    });
}

#[test_connector]
fn a_default_can_be_dropped(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model User {
            id   BigInt     @id @default(autoincrement())
            name String  @default("Musti")
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model User {
            id   BigInt     @id @default(autoincrement())
            name String?
        }
    "#,
    );

    api.create_migration("second-migration", &dm2, &directory).send_sync();

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
        id          BigInt    @id @default(autoincrement())
        lastName    String    @default("")
        password    String?
        updatedAt   DateTime  @default(dbgenerated())
    }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.schema_push_w_datasource(dm1)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(Mysql))]
fn schemas_with_empty_dbgenerated_work_together_with_time_native_type(api: TestApi) {
    // https://github.com/prisma/prisma/issues/16340

    let dm1 = indoc::indoc! {r#"
        model Class {
          id    Int      @id
          when  DateTime @default(dbgenerated()) @db.Time
        }
    "#};

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.schema_push_w_datasource(dm1)
        .send()
        .assert_green()
        .assert_no_steps();
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

    api.schema_push_w_datasource(dm1).send().assert_green();
}

#[test_connector]
fn column_defaults_must_be_migrated(api: TestApi) {
    let dm1 = r#"
        model Fruit {
            id Int @id
            name String @default("banana")
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("Fruit", |table| {
        table.assert_column("name", |col| {
            col.assert_default_kind(Some(DefaultKind::Value(PrismaValue::String("banana".to_string()))))
        })
    });

    let dm2 = r#"
        model Fruit {
            id Int @id
            name String @default("mango")
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Fruit", |table| {
        table.assert_column("name", |col| {
            col.assert_default_kind(Some(DefaultKind::Value(PrismaValue::String("mango".to_string()))))
        })
    });
}

#[test_connector(tags(Mssql))]
fn default_constraint_names_should_work(api: TestApi) {
    let dm = r#"
        generator js {
            provider = "prisma-client-js"
        }

        model A {
            id Int @id @default(autoincrement())
            data String @default("beeb buub", map: "meow")
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("data", |col| {
            let mut expected = DefaultValue::value("beeb buub");
            expected.set_constraint_name("meow");

            col.assert_default(Some(expected))
        })
    });
}

#[test_connector(tags(Mssql))]
fn default_constraint_name_default_values_should_work(api: TestApi) {
    let dm = r#"
        generator js {
            provider = "prisma-client-js"
        }

        model A {
            id Int @id @default(autoincrement())
            data String @default("beeb buub")
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table.assert_column("data", |col| {
            let mut expected = DefaultValue::value("beeb buub");
            expected.set_constraint_name("A_data_df");

            col.assert_default(Some(expected))
        })
    });
}

#[test_connector(tags(Mssql))]
fn default_constraint_name_default_values_with_mapping_should_work(api: TestApi) {
    let dm = r#"
        generator js {
            provider = "prisma-client-js"
        }

        model A {
            id Int @id @default(autoincrement())
            data String @default("beeb buub") @map("purr")

            @@map("meow")
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("meow", |table| {
        table.assert_column("purr", |col| {
            let mut expected = DefaultValue::value("beeb buub");
            expected.set_constraint_name("meow_purr_df");

            col.assert_default(Some(expected))
        })
    });
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

    api.schema_push_w_datasource(dm1)
        .migration_id(Some("first migration"))
        .send()
        .assert_green();

    let insert = Insert::single_into(api.render_table_name("Fruit"))
        .value("id", "apple-id")
        .value("sideNames", "stem and the other one")
        .value("contains", "'vitamin C'")
        .value("seasonality", "september");

    api.query(insert.into());

    api.schema_push_w_datasource(dm1)
        .migration_id(Some("second migration"))
        .send()
        .assert_green()
        .assert_no_steps();

    let sql_schema = api.assert_schema().into_schema();
    let table_id = sql_schema.table_walker(&api.normalize_identifier("Fruit")).unwrap().id;

    if api.is_mssql() {
        let default = sql_schema
            .walk(table_id)
            .column("sideNames")
            .unwrap()
            .default()
            .unwrap();
        assert_eq!(DefaultValue::value("top\ndown").kind(), default.kind());
        assert!(default
            .constraint_name()
            .map(|cn| cn.starts_with("Fruit_sideNames_df"))
            .unwrap());

        let default = sql_schema.walk(table_id).column("contains").unwrap().default().unwrap();
        assert_eq!(DefaultValue::value("'potassium'").kind(), default.kind());
        assert!(default
            .constraint_name()
            .map(|cn| cn.starts_with("Fruit_contains_df"))
            .unwrap());

        let default = sql_schema
            .walk(table_id)
            .column("seasonality")
            .unwrap()
            .default()
            .unwrap();
        assert_eq!(DefaultValue::value(r#""summer""#).kind(), default.kind());
        assert!(default
            .constraint_name()
            .map(|cn| cn.starts_with("Fruit_seasonality_df"))
            .unwrap());
    } else {
        assert_eq!(
            sql_schema
                .walk(table_id)
                .column("sideNames")
                .unwrap()
                .default()
                .map(|d| d.inner()),
            Some(&DefaultValue::value(PrismaValue::String("top\ndown".to_string())))
        );
        assert_eq!(
            sql_schema
                .walk(table_id)
                .column("contains")
                .unwrap()
                .default()
                .map(|d| d.inner()),
            Some(&DefaultValue::value(PrismaValue::String("'potassium'".to_string())))
        );
        assert_eq!(
            sql_schema
                .walk(table_id)
                .column("seasonality")
                .unwrap()
                .default()
                .map(|d| d.inner()),
            Some(&DefaultValue::value(PrismaValue::String(r#""summer""#.to_string())))
        );
    }
}
