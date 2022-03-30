/// Test cockroachdb failure modes
mod failure_modes;

use migration_engine_tests::test_api::*;
use std::fmt::Write as _;

#[test_connector(tags(CockroachDb))]
fn soft_resets_work_on_cockroachdb(mut api: TestApi) {
    let initial = r#"
        CREATE TABLE "Cat" ( id TEXT PRIMARY KEY, name TEXT, meowmeow BOOLEAN );
        CREATE VIEW "catcat" AS SELECT name, meowmeow FROM "Cat" LIMIT 2;
    "#;

    api.raw_cmd(initial);
    api.assert_schema().assert_tables_count(1).assert_has_table("Cat");
    api.reset().soft(true).send_sync();
    api.assert_schema().assert_tables_count(0);
}

#[test_connector(tags(CockroachDb))]
fn cockroach_apply_migrations_errors(api: TestApi) {
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

    "#]];
    let first_segment = err.split_terminator("DbError {").next().unwrap();
    expectation.assert_eq(first_segment)
}

#[test_connector(tags(CockroachDb))]
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
        ("jsonb", "Json", "JsonB", "jsonb"),
        ("inet", "String", "Inet", "inet"),
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

// taken from id tests
#[test_connector(tags(CockroachDb))]
fn moving_the_pk_to_an_existing_unique_constraint_works(api: TestApi) {
    let dm = r#"
        model model1 {
            id              String        @id @default(cuid())
            a               String
            b               String
            c               String

            @@unique([a, b, c])

        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("model1", |table| {
        table
            .assert_pk(|pk| pk.assert_columns(&["id"]))
            .assert_index_on_columns(&["a", "b", "c"], |idx| idx.assert_is_unique())
    });

    api.insert("model1")
        .value("id", "the-id")
        .value("a", "the-a")
        .value("b", "the-b")
        .value("c", "the-c")
        .result_raw();

    let dm2 = r#"
        model model1 {
            id              String
            a               String
            b               String
            c               String

            @@id([a, b, c])

        }
    "#;

    api.schema_push_w_datasource(dm2).force(true).send().assert_green();

    api.assert_schema().assert_table("model1", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["a", "b", "c"]))
    });
}

#[test_connector(tags(CockroachDb))]
fn primary_key_column_type_migrations_are_unexecutable(api: TestApi) {
    let dm1 = r#"
        model Dog {
            name            String
            passportNumber  Int
            p               Puppy[]

            @@id([name, passportNumber])
        }

        model Puppy {
            id String @id
            motherName String
            motherPassportNumber Int
            mother Dog @relation(fields: [motherName, motherPassportNumber], references: [name, passportNumber])
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Dog")
        .value("name", "Marnie")
        .value("passportNumber", 8000)
        .result_raw();

    api.insert("Puppy")
        .value("id", "12345")
        .value("motherName", "Marnie")
        .value("motherPassportNumber", 8000)
        .result_raw();

    // Make Dog#passportNumber a String.
    let dm2 = r#"
        model Dog {
            name           String
            passportNumber String
            p              Puppy[]


            @@id([name, passportNumber])
        }

        model Puppy {
            id String @id
            motherName String
            motherPassportNumber String
            mother Dog @relation(fields: [motherName, motherPassportNumber], references: [name, passportNumber])
        }
    "#;

    let expected_unexecutable = expect![[r#"
        [
            "Changed the type of `passportNumber` on the `Dog` table. No cast exists, the column would be dropped and recreated, which cannot be done since the column is required and there is data in the table.",
        ]
    "#]];

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .expect_unexecutable(expected_unexecutable)
        .assert_warnings(&[]);

    api.assert_schema().assert_table("Dog", |table| {
        table.assert_pk(|pk| pk.assert_columns(&["name", "passportNumber"]))
    });
}
