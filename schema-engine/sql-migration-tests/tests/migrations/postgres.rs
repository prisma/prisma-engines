mod extensions;
mod introspection;
mod multi_schema;

use psl::parser_database::SourceFile;
use quaint::Value;
use schema_core::{json_rpc::types::SchemasContainer, schema_connector::DiffTarget};
use sql_migration_tests::test_api::*;
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

// Reference for the tables created by PostGIS: https://postgis.net/docs/manual-1.4/ch04.html#id418599
#[test_connector(tags(Postgres))]
fn existing_postgis_tables_must_not_be_migrated(api: TestApi) {
    let create_tables = r#"
        CREATE TABLE IF NOT EXISTS "spatial_ref_sys" ( id SERIAL PRIMARY KEY );
        /* The capitalized Geometry is intentional here, because we want the matching to be case-insensitive. */
        CREATE TABLE IF NOT EXISTS "Geometry_columns" ( id SERIAL PRIMARY KEY );
        CREATE TABLE IF NOT EXISTS "geography_columns" ( id SERIAL PRIMARY KEY );
        CREATE TABLE IF NOT EXISTS "raster_columns" ( id SERIAL PRIMARY KEY );
        CREATE TABLE IF NOT EXISTS "raster_overviews" ( id SERIAL PRIMARY KEY );
    "#;

    api.raw_cmd(create_tables);
    api.schema_push_w_datasource("").send().assert_green().assert_no_steps();

    api.assert_schema()
        .assert_has_table("spatial_ref_sys")
        .assert_has_table("Geometry_columns")
        .assert_has_table("geography_columns")
        .assert_has_table("raster_columns")
        .assert_has_table("raster_overviews");
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
        writeln!(&mut dm, "    {field_name} {prisma_type} @db.{native_type}").unwrap();
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

    let migration = api.connector_diff(
        DiffTarget::Database,
        DiffTarget::Datamodel(vec![(
            "schema.prisma".to_string(),
            SourceFile::new_static(target_schema),
        )]),
        None,
    );
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
        let cat_inserts = quaint::ast::Insert::multi_into(api.render_table_name("Cat"), ["id", "mood"])
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

    let engine = schema_core::schema_api(None, None).unwrap();
    let err = tok(
        engine.ensure_connection_validity(schema_core::json_rpc::types::EnsureConnectionValidityParams {
            datasource: schema_core::json_rpc::types::DatasourceParam::Schema(SchemasContainer {
                files: vec![SchemaContainer {
                    path: "schema.prisma".to_string(),
                    content: dm.to_owned(),
                }],
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
            string String[] @default(["Arrabbiata", "Carbonara", "Al Ragù"])
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

    let expected_sql = expect![[r#"
        -- CreateEnum
        CREATE TYPE "Color" AS ENUM ('RED', 'GREEN', 'BLUE');

        -- CreateTable
        CREATE TABLE "Model" (
            "id" INTEGER NOT NULL,
            "int_empty" INTEGER[] DEFAULT ARRAY[]::INTEGER[],
            "int" INTEGER[] DEFAULT ARRAY[0, 1, 1, 2, 3, 5, 8, 13, 21]::INTEGER[],
            "float" DOUBLE PRECISION[] DEFAULT ARRAY[3.20, 4.20, 3.14, 0, 9.9999999, 1000.7]::DOUBLE PRECISION[],
            "string" TEXT[] DEFAULT ARRAY['Arrabbiata', 'Carbonara', 'Al Ragù']::TEXT[],
            "boolean" BOOLEAN[] DEFAULT ARRAY[false, true, true, true]::BOOLEAN[],
            "dateTime" TIMESTAMP(3)[] DEFAULT ARRAY['2019-06-17 14:20:57 +00:00', '2020-09-21 20:00:00 +02:00']::TIMESTAMP(3)[],
            "colors" "Color"[] DEFAULT ARRAY['GREEN', 'BLUE']::"Color"[],
            "colors_empty" "Color"[] DEFAULT ARRAY[]::"Color"[],
            "bytes" BYTEA[] DEFAULT ARRAY['\x68656c6c6f20776f726c64']::BYTEA[],
            "json" JSONB[] DEFAULT ARRAY['{ "a": ["b"] }', '3']::JSONB[],
            "decimal" DECIMAL(65,30)[] DEFAULT ARRAY[121.10299000124800000001, 0.4, 1.1, -68.0]::DECIMAL(65,30)[],

            CONSTRAINT "Model_pkey" PRIMARY KEY ("id")
        );
    "#]];

    api.expect_sql_for_schema(schema, &expected_sql);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn scalar_list_default_diffing(api: TestApi) {
    let schema_1 = r#"
        datasource db {
          provider = "postgresql"
          url = env("DATABASE_URL")
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
            string String[] @default(["Arrabbiata", "Carbonara", "Al Ragù"])
            boolean Boolean[] @default([false, true ,true, true])
            dateTime DateTime[] @default(["2019-06-17T14:20:57Z", "2020-09-21T20:00:00+02:00"])
            colors Color[] @default([GREEN, BLUE])
            colors_empty Color[] @default([])
            bytes    Bytes[] @default(["aGVsbG8gd29ybGQ="])
            json     Json[]  @default(["{ \"a\": [\"b\"] }", "3"])
            decimal  Decimal[]  @default(["121.10299000124800000001", "0.4", "1.1", "-68.0"])
        }
    "#;

    let schema_2 = r#"
        datasource db {
          provider = "postgresql"
          url = env("DATABASE_URL")
        }

        enum Color {
            RED
            GREEN
            BLUE
        }

        model Model {
            id Int @id
            int_empty Int[] @default([])
            int Int[] @default([0, 1, 1, 2, 3, 5, 8, 13, 22])
            float Float[] @default([3.20, 4.20, 9.9999999, 1000.7])
            string String[] @default(["Arrabbiata", "Quattro Formaggi","Al Ragù"])
            boolean Boolean[] @default([true, true ,true, true])
            dateTime DateTime[] @default(["2019-06-17T14:20:57Z", "2020-09-21T20:00:00+02:00"])
            colors Color[] @default([BLUE, GREEN])
            colors_empty Color[] @default([])
            bytes    Bytes[] @default(["aGVsbG8gd29ybGQ=", "aGVsbG8gd37ybGQ="])
            json     Json[]  @default(["{ \"a\": [\"b\"] }", "4"])
            decimal  Decimal[]  @default(["0.4", "1.1", "-68.0"])
        }
    "#;

    let migration = api.connector_diff(
        DiffTarget::Datamodel(vec![("schema.prisma".to_string(), SourceFile::new_static(schema_1))]),
        DiffTarget::Datamodel(vec![("schema.prisma".to_string(), SourceFile::new_static(schema_2))]),
        None,
    );

    let expected_migration = expect![[r#"
        -- AlterTable
        ALTER TABLE "Model" ALTER COLUMN "int" SET DEFAULT ARRAY[0, 1, 1, 2, 3, 5, 8, 13, 22]::INTEGER[],
        ALTER COLUMN "float" SET DEFAULT ARRAY[3.20, 4.20, 9.9999999, 1000.7]::DOUBLE PRECISION[],
        ALTER COLUMN "string" SET DEFAULT ARRAY['Arrabbiata', 'Quattro Formaggi', 'Al Ragù']::TEXT[],
        ALTER COLUMN "boolean" SET DEFAULT ARRAY[true, true, true, true]::BOOLEAN[],
        ALTER COLUMN "colors" SET DEFAULT ARRAY['BLUE', 'GREEN']::"Color"[],
        ALTER COLUMN "bytes" SET DEFAULT ARRAY['\x68656c6c6f20776f726c64', '\x68656c6c6f20777ef26c64']::BYTEA[],
        ALTER COLUMN "json" SET DEFAULT ARRAY['{ "a": ["b"] }', '4']::JSONB[],
        ALTER COLUMN "decimal" SET DEFAULT ARRAY[0.4, 1.1, -68.0]::DECIMAL(65,30)[];
    "#]];

    expected_migration.assert_eq(&migration);

    api.schema_push(schema_1).send().assert_green();
    api.schema_push(schema_1).send().assert_green().assert_no_steps();
    api.schema_push(schema_2)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push(schema_2).send().assert_green().assert_no_steps();
}

// https://github.com/prisma/prisma/issues/12095
#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn json_defaults_with_escaped_quotes_work(api: TestApi) {
    let schema = r#"
        datasource db {
          provider = "postgresql"
          url      = env("DATABASE_URL")
        }

        model Foo {
          id             Int   @id
          bar Json? @default("{\"message\": \"This message includes a quote: Here''s it!\"}")
        }
    "#;

    api.schema_push(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push(schema).send().assert_green().assert_no_steps();

    let sql = expect![[r#"
        -- CreateTable
        CREATE TABLE "Foo" (
            "id" INTEGER NOT NULL,
            "bar" JSONB DEFAULT '{"message": "This message includes a quote: Here''''s it!"}',

            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );
    "#]];

    api.expect_sql_for_schema(schema, &sql);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn bigint_defaults_work(api: TestApi) {
    let schema = r#"
        datasource mypg {
            provider = "postgresql"
            url = env("TEST_DATABASE_URL")
        }

        model foo {
          id  String @id
          bar BigInt @default(0)
        }
    "#;
    let sql = expect![[r#"
        -- CreateTable
        CREATE TABLE "foo" (
            "id" TEXT NOT NULL,
            "bar" BIGINT NOT NULL DEFAULT 0,

            CONSTRAINT "foo_pkey" PRIMARY KEY ("id")
        );
    "#]];
    api.expect_sql_for_schema(schema, &sql);

    api.schema_push(schema).send().assert_green();
    api.schema_push(schema).send().assert_green().assert_no_steps();
}

// https://github.com/prisma/prisma/issues/14799
#[test_connector(tags(Postgres12), exclude(CockroachDb))]
fn dbgenerated_on_generated_columns_is_idempotent(api: TestApi) {
    let sql = r#"
        CREATE TABLE "table" (
         "id" TEXT NOT NULL,
         "hereBeDragons" TEXT NOT NULL GENERATED ALWAYS AS ('this row ID is: '::text || "id") STORED,

         CONSTRAINT "table_pkey" PRIMARY KEY ("id")
        );
    "#;

    api.raw_cmd(sql);

    let schema = r#"
        datasource db {
            provider = "postgresql"
            url = env("TEST_DATABASE_URL")
        }

        model table {
            id String @id
            hereBeDragons String @default(dbgenerated())
        }
    "#;

    api.schema_push(schema).send().assert_green().assert_no_steps();
}

// https://github.com/prisma/prisma/issues/15654
#[test_connector(tags(Postgres12), exclude(CockroachDb))]
fn dbgenerated_on_generated_unsupported_columns_is_idempotent(api: TestApi) {
    let sql = r#"
        CREATE TABLE "table" (
            "id" TEXT NOT NULL,
            -- NOTE: Modified to make it a PG generated column
            "hereBeDragons" tsvector GENERATED ALWAYS AS (
                to_tsvector('english', id::text)
            ) STORED,

            CONSTRAINT "table_pkey" PRIMARY KEY ("id")
        );
    "#;

    api.raw_cmd(sql);

    let schema = r#"
        datasource db {
            provider = "postgresql"
            url = env("TEST_DATABASE_URL")
        }

        model table {
            id String @id
            hereBeDragons Unsupported("tsvector")? @default(dbgenerated())
        }
    "#;

    api.schema_push(schema).send().assert_green().assert_no_steps();
}
