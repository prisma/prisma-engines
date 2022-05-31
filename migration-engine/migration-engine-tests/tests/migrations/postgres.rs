use migration_core::migration_connector::DiffTarget;
use migration_engine_tests::test_api::*;
use quaint::Value;
use sql_schema_describer::ColumnTypeFamily;
use std::fmt::Write;

#[test_connector(tags(Postgres))]
fn enums_can_be_dropped_on_postgres(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id String @id
            name String
            mood CatMood
        }

        enum CatMood {
            ANGRY
            HUNGRY
            CUDDLY
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema()
        .assert_enum("CatMood", |r#enum| r#enum.assert_values(&["ANGRY", "HUNGRY", "CUDDLY"]));

    let dm2 = r#"
        model Cat {
            id String @id
            name String
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();
    api.assert_schema().assert_has_no_enum("CatMood");
}

#[test_connector(capabilities(ScalarLists))]
fn adding_a_scalar_list_for_a_model_with_id_type_int_must_work(api: TestApi) {
    let dm1 = r#"
        model A {
            id Int @id
            strings String[]
            enums Status[]
        }

        enum Status {
            OK
            ERROR
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("strings", |col| col.assert_is_list().assert_type_is_string())
            .assert_column("enums", |col| {
                col.assert_type_family(ColumnTypeFamily::Enum("Status".into()))
                    .assert_is_list()
            })
    });
}

// Reference for the tables created by PostGIS: https://postgis.net/docs/manual-1.4/ch04.html#id418599
#[test_connector(tags(Postgres))]
fn existing_postgis_tables_must_not_be_migrated(api: TestApi) {
    let create_tables = r#"
        CREATE TABLE IF NOT EXISTS "spatial_ref_sys" ( id SERIAL PRIMARY KEY );
        /* The capitalized Geometry is intentional here, because we want the matching to be case-insensitive. */
        CREATE TABLE IF NOT EXISTS "Geometry_columns" ( id SERIAL PRIMARY KEY );
    "#;

    api.raw_cmd(create_tables);
    api.schema_push_w_datasource("").send().assert_green().assert_no_steps();

    api.assert_schema()
        .assert_has_table("spatial_ref_sys")
        .assert_has_table("Geometry_columns");
}

// Reference for the views created by PostGIS: https://postgis.net/docs/manual-1.4/ch04.html#id418599
#[test_connector(tags(Postgres))]
fn existing_postgis_views_must_not_be_migrated(api: TestApi) {
    let create_views = r#"
        CREATE VIEW "spatial_ref_sys" AS SELECT 1;
        /* The capitalized Geometry is intentional here, because we want the matching to be case-insensitive. */
        CREATE VIEW "Geometry_columns" AS SELECT 1;
        CREATE VIEW "PG_BUFFERCACHE" AS SELECT 1;
    "#;

    api.raw_cmd(create_views);
    api.schema_push_w_datasource("").send().assert_green().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn native_type_columns_can_be_created(api: TestApi) {
    let types = &[
        ("smallint", "Int", "SmallInt", "int2"),
        ("int", "Int", "Integer", "int4"),
        ("bigint", "BigInt", "BigInt", "int8"),
        ("decimal", "Decimal", "Decimal(4, 2)", "numeric"),
        ("decimaldefault", "Decimal", "Decimal", "numeric"),
        ("real", "Float", "Real", "float4"),
        ("doublePrecision", "Float", "DoublePrecision", "float8"),
        ("varChar", "String", "VarChar(200)", "varchar"),
        ("char", "String", "Char(200)", "bpchar"),
        ("text", "String", "Text", "text"),
        ("bytea", "Bytes", "ByteA", "bytea"),
        ("ts", "DateTime", "Timestamp(0)", "timestamp"),
        ("tsdefault", "DateTime", "Timestamp", "timestamp"),
        ("tstz", "DateTime", "Timestamptz", "timestamptz"),
        ("date", "DateTime", "Date", "date"),
        ("time", "DateTime", "Time(2)", "time"),
        ("timedefault", "DateTime", "Time", "time"),
        ("timetz", "DateTime", "Timetz(2)", "timetz"),
        ("timetzdefault", "DateTime", "Timetz", "timetz"),
        ("bool", "Boolean", "Boolean", "bool"),
        ("bit", "String", "Bit(1)", "bit"),
        ("varbit", "String", "VarBit(1)", "varbit"),
        ("uuid", "String", "Uuid", "uuid"),
        ("xml", "String", "Xml", "xml"),
        ("json", "Json", "Json", "json"),
        ("jsonb", "Json", "JsonB", "jsonb"),
        ("money", "Decimal", "Money", "money"),
        ("inet", "String", "Inet", "inet"),
        ("oid", "Int", "Oid", "oid"),
    ];

    let mut dm = r#"
        model A {
            id Int @id
    "#
    .to_string();

    for (field_name, prisma_type, native_type, _) in types {
        writeln!(&mut dm, "    {} {} @db.{}", field_name, prisma_type, native_type).unwrap();
    }

    dm.push_str("}\n");

    api.schema_push_w_datasource(&dm).send().assert_green();

    api.assert_schema().assert_table("A", |table| {
        types.iter().fold(
            table,
            |table, (field_name, _prisma_type, _native_type, database_type)| {
                table.assert_column(field_name, |col| col.assert_full_data_type(database_type))
            },
        )
    });

    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Postgres))]
fn uuids_do_not_generate_drift_issue_5282(api: TestApi) {
    if !api.is_cockroach() {
        api.raw_cmd(r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp";"#)
    }

    api.raw_cmd(
        r#"
        CREATE TABLE a (id uuid DEFAULT uuid_generate_v4() primary key);
        CREATE TABLE b (id uuid DEFAULT uuid_generate_v4() primary key, a_id uuid, CONSTRAINT aaa FOREIGN KEY (a_id) REFERENCES a(id) ON DELETE SET NULL ON UPDATE CASCADE);
        "#
    );

    let dm = r#"
        model a {
            id String @id(map: "a_pkey") @default(dbgenerated("uuid_generate_v4()")) @db.Uuid
            b  b[]
        }

        model b {
            id   String  @id(map: "b_pkey") @default(dbgenerated("uuid_generate_v4()")) @db.Uuid
            a_id String? @db.Uuid
            a    a?      @relation(fields: [a_id], references: [id], map: "aaa")
        }
        "#;

    api.schema_push_w_datasource(dm)
        .migration_id(Some("first"))
        .send()
        .assert_green()
        .assert_no_steps();
}

// CockroachDB does not support uuid-ossp functions in a separate schema.
#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn functions_with_schema_prefix_in_dbgenerated_are_idempotent(api: TestApi) {
    api.raw_cmd(r#"CREATE SCHEMA "myschema"; CREATE EXTENSION IF NOT EXISTS "uuid-ossp" WITH SCHEMA "myschema";"#);

    let dm = r#"
        model Koala {
            id String @id @db.Uuid @default(dbgenerated("myschema.uuid_generate_v4()"))
        }
        "#;

    api.schema_push_w_datasource(dm)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn postgres_apply_migrations_errors_give_precise_location(api: TestApi) {
    let dm = "";
    let migrations_directory = api.create_migrations_directory();

    let migration = r#"
        CREATE TABLE "Cat" (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        );

        SELECT id FROM "Dog";

        CREATE TABLE "Emu" (
            size INTEGER
        );
    "#;

    let migration_name = api
        .create_migration("01init", dm, &migrations_directory)
        .draft(true)
        .send_sync()
        .modify_migration(|contents| {
            contents.clear();
            contents.push_str(migration);
        })
        .into_output()
        .generated_migration_name
        .unwrap();

    let err = api
        .apply_migrations(&migrations_directory)
        .send_unwrap_err()
        .to_string()
        .replace(&migration_name, "<migration-name>");

    let expectation = expect![[r#"
        A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

        Migration name: <migration-name>

        Database error code: 42P01

        Database error:
        ERROR: relation "Dog" does not exist

        Position:
        [1m  2[0m         CREATE TABLE "Cat" (
        [1m  3[0m             id INTEGER PRIMARY KEY,
        [1m  4[0m             name TEXT NOT NULL
        [1m  5[0m         );
        [1m  6[0m
        [1m  7[1;31m         SELECT id FROM "Dog";[0m

    "#]];
    let first_segment = err.split_terminator("DbError {").next().unwrap();
    expectation.assert_eq(first_segment)
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn postgres_apply_migrations_errors_give_precise_location_at_the_beginning_of_files(api: TestApi) {
    let dm = "";
    let migrations_directory = api.create_migrations_directory();

    let migration = r#"
        CREATE TABLE "Cat" ( id INTEGER PRIMARY KEY );

        SELECT id FROM "Dog";

        CREATE TABLE "Emu" (
            size INTEGER
        );
    "#;

    let migration_name = api
        .create_migration("01init", dm, &migrations_directory)
        .draft(true)
        .send_sync()
        .modify_migration(|contents| {
            contents.clear();
            contents.push_str(migration);
        })
        .into_output()
        .generated_migration_name
        .unwrap();

    let err = api
        .apply_migrations(&migrations_directory)
        .send_unwrap_err()
        .to_string()
        .replace(&migration_name, "<migration-name>");

    let expectation = expect![[r#"
        A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

        Migration name: <migration-name>

        Database error code: 42P01

        Database error:
        ERROR: relation "Dog" does not exist

        Position:
        [1m  0[0m
        [1m  1[0m
        [1m  2[0m         CREATE TABLE "Cat" ( id INTEGER PRIMARY KEY );
        [1m  3[0m
        [1m  4[1;31m         SELECT id FROM "Dog";[0m

    "#]];
    let first_segment = err.split_terminator("DbError {").next().unwrap();
    expectation.assert_eq(first_segment)
}

// exclude: CITEXT does not exist on cockroachdb at this point in time.
#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn citext_to_text_and_back_works(api: TestApi) {
    api.raw_cmd("CREATE EXTENSION citext;");

    let dm1 = r#"
        model User {
            id Int @id @default(autoincrement())
            name String @db.Text
        }
    "#;

    let dm2 = r#"
        model User {
            id Int @id @default(autoincrement())
            name String @db.Citext
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.raw_cmd("INSERT INTO \"User\" (name) VALUES ('myCat'), ('myDog'), ('yourDog');");

    // TEXT -> CITEXT
    api.schema_push_w_datasource(dm2)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.dump_table("User")
        .assert_row_count(3)
        .assert_first_row(|row| row.assert_text_value("name", "myCat"));

    // CITEXT -> TEXT
    api.schema_push_w_datasource(dm1)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.dump_table("User")
        .assert_row_count(3)
        .assert_first_row(|row| row.assert_text_value("name", "myCat"));
}

#[test_connector(tags(Postgres))]
fn foreign_key_renaming_to_default_works(api: TestApi) {
    let setup = r#"
        CREATE TABLE "food" (
            id BIGSERIAL PRIMARY KEY
        );

        CREATE TABLE "Dog" (
            id BIGSERIAL PRIMARY KEY,
            favourite_food_id BIGINT NOT NULL,
            CONSTRAINT "favouriteFood" FOREIGN KEY ("favourite_food_id")
                    REFERENCES "food"("id")
                    ON DELETE RESTRICT
                    ON UPDATE CASCADE
        );
    "#;

    api.raw_cmd(setup);

    let target_schema = r#"
        datasource db {
            provider = "postgresql"
            url = env("TEST_DATABASE_URL")
        }

        model Dog {
            id                  BigInt @id @default(autoincrement())
            favourite_food_id   BigInt
            favouriteFood       food @relation(fields: [favourite_food_id], references: [id], onDelete: Restrict, onUpdate: Cascade)
        }

        model food {
            id      BigInt @id @default(autoincrement())
            dogs    Dog[]
        }
    "#;

    let migration = api.connector_diff(DiffTarget::Database, DiffTarget::Datamodel(target_schema));
    let expected = expect![[r#"
        -- RenameForeignKey
        ALTER TABLE "Dog" RENAME CONSTRAINT "favouriteFood" TO "Dog_favourite_food_id_fkey";
    "#]];

    expected.assert_eq(&migration);

    // Check that the migration applies cleanly.
    api.raw_cmd(&migration);

    // Check that the migration is idempotent.
    api.schema_push(target_schema).send().assert_green().assert_no_steps();
}

// exclude: enum migrations work differently on cockroachdb, there is no migration
#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn failing_enum_migrations_should_not_be_partially_applied(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id String @id
            mood Mood
        }

        enum Mood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push_w_datasource(dm1)
        .migration_id(Some("initial-setup"))
        .send()
        .assert_green();

    {
        let cat_inserts = quaint::ast::Insert::multi_into(api.render_table_name("Cat"), &["id", "mood"])
            .values((Value::text("felix"), Value::enum_variant("HUNGRY")))
            .values((Value::text("mittens"), Value::enum_variant("HAPPY")));

        api.query(cat_inserts.into());
    }

    let dm2 = r#"
        model Cat {
            id   String @id
            mood Mood
        }

        enum Mood {
            HUNGRY
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .migration_id(Some("remove-used-variant"))
        .force(true)
        .send_unwrap_err();

    // Assertions
    {
        api.raw_cmd("ROLLBACK");

        let cat_data = api.dump_table("Cat");
        let cat_data: Vec<Vec<quaint::ast::Value>> =
            cat_data.into_iter().map(|row| row.into_iter().collect()).collect();

        let expected_cat_data = vec![
            vec![Value::text("felix"), Value::enum_variant("HUNGRY")],
            vec![Value::text("mittens"), Value::enum_variant("HAPPY")],
        ];

        assert_eq!(cat_data, expected_cat_data);

        if api.is_mysql() {
            api.assert_schema()
                .assert_enum("Cat_mood", |enm| enm.assert_values(&["HAPPY", "HUNGRY"]));
        } else {
            api.assert_schema()
                .assert_enum("Mood", |enm| enm.assert_values(&["HAPPY", "HUNGRY"]));
        };
    }
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn connecting_to_a_postgres_database_with_the_cockroach_connector_fails(_api: TestApi) {
    let dm = r#"
        datasource crdb {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }
    "#;

    let engine = migration_core::migration_api(None, None).unwrap();
    let err = tok(
        engine.ensure_connection_validity(migration_core::json_rpc::types::EnsureConnectionValidityParams {
            datasource: migration_core::json_rpc::types::DatasourceParam::SchemaString(SchemaContainer {
                schema: dm.to_owned(),
            }),
        }),
    )
    .unwrap_err()
    .to_string();

    let expected_error = expect![[r#"
        You are trying to connect to a PostgreSQL database, but the provider in your Prisma schema is `cockroachdb`. Please change it to `postgresql`.
    "#]];
    expected_error.assert_eq(&err);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn scalar_list_defaults_work(api: TestApi) {
    let schema = r#"
        datasource db {
          provider = "postgresql"
          url = "postgres://"
        }

        enum Color {
            RED
            GREEN
            BLUE
        }

        model Model {
            id Int @id
            int_empty Int[] @default([])
            int Int[] @default([0, 1, 1, 2, 3, 5, 8, 13, 21])
            float Float[] @default([3.20, 4.20, 3.14, 0, 9.9999999, 1000.7])
            string String[] @default(["Arrabiata", "Carbonara", "Al Ragù"])
            boolean Boolean[] @default([false, true ,true, true])
            dateTime DateTime[] @default(["2019-06-17T14:20:57Z", "2020-09-21T20:00:00+02:00"])
            colors Color[] @default([GREEN, BLUE])
            colors_empty Color[] @default([])
            bytes    Bytes[] @default(["aGVsbG8gd29ybGQ="])
            json     Json[]  @default(["{ \"a\": [\"b\"] }", "3"])
            decimal  Decimal[]  @default(["121.10299000124800000001", "0.4", "1.1", "-68.0"])
        }
    "#;

    api.schema_push(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push(schema).send().assert_green().assert_no_steps();
}
