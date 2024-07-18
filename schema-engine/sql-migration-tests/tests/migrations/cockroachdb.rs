/// Test cockroachdb failure modes
mod failure_modes;

use prisma_value::PrismaValue;
use psl::parser_database::*;
use quaint::prelude::Insert;
use schema_core::{json_rpc::types::SchemasContainer, schema_connector::DiffTarget};
use serde_json::json;
use sql_migration_tests::test_api::*;
use sql_schema_describer::{ColumnTypeFamily, ForeignKeyAction};
use std::fmt::Write as _;

#[test_connector(tags(CockroachDb))]
fn db_push_on_cockroach_db_with_postgres_provider_fails(api: TestApi) {
    let schema = format!(
        r#"
        datasource mypg {{
            provider = "postgresql"
            url = "{}"
        }}

        model Test {{
            id      Int @id
            name    String
        }}
    "#,
        api.connection_string()
    );

    let connector = schema_core::schema_api(Some(schema.clone()), None).unwrap();
    let error = tok(connector.schema_push(schema_core::json_rpc::types::SchemaPushInput {
        force: false,
        schema: schema_core::json_rpc::types::SchemasContainer {
            files: vec![schema_core::json_rpc::types::SchemaContainer {
                path: "schema.prisma".to_string(),
                content: schema,
            }],
        },
    }))
    .unwrap_err()
    .message()
    .unwrap()
    .to_owned();

    let expected_err = expect![
        r#"You are trying to connect to a CockroachDB database, but the provider in your Prisma schema is `postgresql`. Please change it to `cockroachdb`."#
    ];

    expected_err.assert_eq(&error);
}

#[test_connector(tags(CockroachDb))]
fn soft_resets_work_on_cockroachdb(mut api: TestApi) {
    let initial = r#"
        CREATE TABLE "Cat" ( id TEXT PRIMARY KEY, name TEXT, meowmeow BOOLEAN );
        CREATE VIEW "catcat" AS SELECT name, meowmeow FROM "Cat" LIMIT 2;
    "#;

    api.raw_cmd(initial);
    api.assert_schema().assert_tables_count(1).assert_has_table("Cat");
    api.reset().soft(true).send_sync(None);
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
        ("smallint", "Int", "Int2", "int2"),
        ("int", "Int", "Int4", "int4"),
        ("bigint", "BigInt", "Int8", "int8"),
        ("oid", "Int", "Oid", "oid"),
        ("decimal", "Decimal", "Decimal(4, 2)", "numeric"),
        ("decimaldefault", "Decimal", "Decimal", "numeric"),
        ("float4col", "Float", "Float4", "float4"),
        ("float8col", "Float", "Float8", "float8"),
        ("stringargs", "String", "String(200)", "text"),
        ("char", "String", "Char(200)", "bpchar"),
        ("singlechar", "String", "CatalogSingleChar", "char"),
        ("stringnoarg", "String", "String", "text"),
        ("bytea", "Bytes", "Bytes", "bytea"),
        ("ts", "DateTime", "Timestamp(0)", "timestamp"),
        ("tsdefault", "DateTime", "Timestamp", "timestamp"),
        ("tstz", "DateTime", "Timestamptz", "timestamptz"),
        ("date", "DateTime", "Date", "date"),
        ("time", "DateTime", "Time(2)", "time"),
        ("timedefault", "DateTime", "Time", "time"),
        ("timetz", "DateTime", "Timetz(2)", "timetz"),
        ("timetzdefault", "DateTime", "Timetz", "timetz"),
        ("bool", "Boolean", "Bool", "bool"),
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

#[test_connector(tags(CockroachDb))]
fn bigint_primary_keys_are_idempotent(api: TestApi) {
    let dm1 = r#"
            model Cat {
                id BigInt @id @default(autoincrement()) @db.Int8
            }
        "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.schema_push_w_datasource(dm1)
        .send()
        .assert_green()
        .assert_no_steps();

    let dm2 = r#"
        model Cat {
            id BigInt @id @default(autoincrement())
        }
        "#;

    api.schema_push_w_datasource(dm2).send().assert_green();
    api.schema_push_w_datasource(dm2)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(CockroachDb))]
fn typescript_starter_schema_with_different_native_types_is_idempotent(api: TestApi) {
    let dm = r#"
        model Post {
            id        BigInt     @id @default(autoincrement())
            title     String
            content   String?
            published Boolean @default(false)
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?
        }

        model User {
            id    Int     @id 
            email String  @unique
            name  String?
            posts Post[]
        }
    "#;

    let dm2 = r#"
        model Post {
            id        BigInt  @id @default(autoincrement()) @db.Int8
            title     String  @db.String(100)
            content   String? @db.String(100)
            published Boolean @default(false) @db.Bool
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?    @db.Int4
        }

        model User {
            id    Int     @id @db.Int4
            email String  @unique @db.String(100)
            name  String? @db.String(100)
            posts Post[]
        }
    "#;

    api.schema_push_w_datasource(dm)
        .migration_id(Some("first"))
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(dm)
        .migration_id(Some("second"))
        .send()
        .assert_green()
        .assert_no_steps();
    api.schema_push_w_datasource(dm2)
        .migration_id(Some("third"))
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(dm2)
        .migration_id(Some("fourth"))
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(CockroachDb))]
fn typescript_starter_schema_with_native_types_is_idempotent(api: TestApi) {
    let dm = r#"
        model Post {
            id        BigInt     @id @default(autoincrement())
            title     String
            content   String?
            published Boolean @default(false)
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  BigInt?
        }

        model User {
            id    BigInt     @id @default(autoincrement())
            email String  @unique
            name  String?
            posts Post[]
        }
    "#;

    let dm2 = r#"
        model Post {
            id        BigInt     @id @default(autoincrement()) @db.Int8
            title     String  @db.String
            content   String? @db.String
            published Boolean @default(false) @db.Bool
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  BigInt?    @db.Int8
        }

        model User {
            id    BigInt     @id @default(autoincrement()) @db.Int8
            email String  @unique @db.String
            name  String? @db.String
            posts Post[]
        }
    "#;

    api.schema_push_w_datasource(dm)
        .migration_id(Some("first"))
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(dm)
        .migration_id(Some("second"))
        .send()
        .assert_green()
        .assert_no_steps();
    api.schema_push_w_datasource(dm2)
        .migration_id(Some("third"))
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(CockroachDb))]
fn connecting_to_a_cockroachdb_database_with_the_postgresql_connector_fails(_api: TestApi) {
    let dm = r#"
        datasource crdb {
            provider = "postgresql"
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
        You are trying to connect to a CockroachDB database, but the provider in your Prisma schema is `postgresql`. Please change it to `cockroachdb`.
    "#]];
    expected_error.assert_eq(&err);
}

// This test follows https://github.com/prisma/prisma-engines/pull/4632.
#[test_connector(tags(CockroachDb))]
fn decimal_to_boolean_migrations_work(api: TestApi) {
    let dir = api.create_migrations_directory();

    let dm1 = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Cat {
            id  BigInt @id @default(autoincrement())
            tag Decimal
        }
    "#;

    api.create_migration("create-cats-decimal", dm1, &dir)
        .send_sync()
        .assert_migration_directories_count(1)
        .assert_migration("create-cats-decimal", move |migration| {
            let expected_script = expect![[r#"
                -- CreateTable
                CREATE TABLE "Cat" (
                    "id" INT8 NOT NULL DEFAULT unique_rowid(),
                    "tag" DECIMAL(65,30) NOT NULL,

                    CONSTRAINT "Cat_pkey" PRIMARY KEY ("id")
                );
            "#]];

            migration.expect_contents(expected_script)
        });

    let dm2 = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }
        
        model Cat {
            id  BigInt @id @default(autoincrement())
            tag Boolean
        }
    "#;

    api.create_migration("migrate-cats-boolean", dm2, &dir)
        .send_sync()
        .assert_migration_directories_count(2)
        .assert_migration("migrate-cats-boolean", move |migration| {
            let expected_script = expect![[r#"
                /*
                  Warnings:

                  - Changed the type of `tag` on the `Cat` table. No cast exists, the column would be dropped and recreated, which cannot be done if there is data, since the column is required.

                */
                -- AlterTable
                ALTER TABLE "Cat" DROP COLUMN "tag";
                ALTER TABLE "Cat" ADD COLUMN     "tag" BOOL NOT NULL;
            "#]];

            migration.expect_contents(expected_script)
        });
}

#[test_connector(tags(CockroachDb))]
fn int_to_string_conversions_work(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id  BigInt @id @default(autoincrement())
            tag Int
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    api.insert("Cat").value("tag", 20).result_raw();

    let dm2 = r#"
        model Cat {
            id  BigInt @id @default(autoincrement())
            tag String
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.dump_table("Cat")
        .assert_single_row(|row| row.assert_text_value("tag", "20"));
}

#[test_connector(tags(CockroachDb))]
fn adding_an_unsupported_type_must_work(api: TestApi) {
    let dm = r#"
        model Post {
            id            Int                     @id
            /// This type is currently not supported.
            user_ip  Unsupported("interval")
            User          User                    @relation(fields: [user_ip], references: [balance])
        }

        model User {
            id            Int                     @id
            /// This type is currently not supported.
            balance       Unsupported("interval")  @unique
            Post          Post[]
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Post", |table| {
        table
            .assert_columns_count(2)
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("user_ip", |c| {
                c.assert_is_required()
                    .assert_type_family(ColumnTypeFamily::Unsupported("interval".to_string()))
            })
    });

    api.assert_schema().assert_table("User", |table| {
        table
            .assert_columns_count(2)
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Int)
            })
            .assert_column("balance", |c| {
                c.assert_is_required()
                    .assert_type_family(ColumnTypeFamily::Unsupported("interval".to_string()))
            })
    });
}

#[test_connector(tags(CockroachDb))]
fn switching_an_unsupported_type_to_supported_must_work(api: TestApi) {
    let dm1 = r#"
        model Post {
            id              BigInt                     @id @default(autoincrement())
            user_home       Unsupported("interval")
            user_location   Unsupported("interval")
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model Post {
            id            BigInt                     @id @default(autoincrement())
            user_home     String
            user_location String
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();
    api.schema_push_w_datasource(dm2).send().assert_no_steps();

    api.assert_schema().assert_table("Post", |table| {
        table
            .assert_columns_count(3)
            .assert_column("id", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::BigInt)
            })
            .assert_column("user_home", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::String)
            })
            .assert_column("user_location", |c| {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::String)
            })
    });
}

#[test_connector(tags(CockroachDb))]
fn column_defaults_can_safely_be_changed(api: TestApi) {
    let combinations = &[
        ("Meow", Some(PrismaValue::String("Cats".to_string())), None),
        ("Freedom", None, Some(PrismaValue::String("Braveheart".to_string()))),
        (
            "OutstandingMovies",
            Some(PrismaValue::String("Cats".to_string())),
            Some(PrismaValue::String("Braveheart".to_string())),
        ),
    ];

    for (model_name, first_default, second_default) in combinations {
        let span = tracing::info_span!("Combination", model_name, ?first_default, ?second_default);
        let _combination_scope = span.enter();
        tracing::info!("Testing new combination");

        // Set up the initial schema
        {
            let dm1 = format!(
                r#"
                    model {} {{
                        id String @id
                        name String? {}
                    }}
                "#,
                model_name,
                first_default
                    .as_ref()
                    .map(|default| format!("@default(\"{default}\")"))
                    .unwrap_or_else(String::new)
            );

            api.schema_push_w_datasource(dm1).force(true).send();

            api.assert_schema().assert_table(model_name, |table| {
                table.assert_column("name", |column| {
                    if let Some(first_default) = first_default.as_ref() {
                        column.assert_default_value(first_default)
                    } else {
                        column.assert_has_no_default()
                    }
                })
            });
        }

        // Insert data
        {
            let insert_span = tracing::info_span!("Data insertion");
            let _insert_scope = insert_span.enter();

            let query = Insert::single_into(api.render_table_name(model_name)).value("id", "abc");

            api.query(query.into());

            let query = Insert::single_into(api.render_table_name(model_name))
                .value("id", "def")
                .value("name", "Waterworld");

            api.query(query.into());

            let data = api.dump_table(model_name);
            let names: Vec<PrismaValue> = data
                .into_iter()
                .filter_map(|row| {
                    row.get("name")
                        .map(|val| val.to_string().map(PrismaValue::String).unwrap_or(PrismaValue::Null))
                })
                .collect();

            assert_eq!(
                &[
                    first_default.as_ref().cloned().unwrap_or(PrismaValue::Null),
                    PrismaValue::String("Waterworld".to_string())
                ],
                names.as_slice()
            );
        }

        // Migrate
        {
            let dm2 = format!(
                r#"
                    model {} {{
                        id String @id
                        name String? {}
                    }}
                "#,
                model_name,
                second_default
                    .as_ref()
                    .map(|default| format!(r#"@default("{default}")"#))
                    .unwrap_or_else(String::new)
            );

            api.schema_push_w_datasource(dm2).send().assert_green();
        }

        // Check that the data is still there
        {
            let data = api.dump_table(model_name);
            let names: Vec<PrismaValue> = data
                .into_iter()
                .filter_map(|row| {
                    row.get("name")
                        .map(|val| val.to_string().map(PrismaValue::String).unwrap_or(PrismaValue::Null))
                })
                .collect();
            assert_eq!(
                &[
                    first_default.as_ref().cloned().unwrap_or(PrismaValue::Null),
                    PrismaValue::String("Waterworld".to_string())
                ],
                names.as_slice()
            );

            api.assert_schema().assert_table(model_name, |table| {
                table.assert_column("name", |column| {
                    if let Some(second_default) = second_default.as_ref() {
                        column.assert_default_value(second_default)
                    } else {
                        column.assert_has_no_default()
                    }
                })
            });
        }
    }
}

#[test_connector(tags(CockroachDb))]
fn removing_autoincrement_from_an_existing_field_works(api: TestApi) {
    use quaint::ast::{Insert, Select};

    let dm1 = r#"
        model Post {
            id        BigInt         @id @default(autoincrement())
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    // Data to see we don't lose anything in the translation.
    for content in &["A", "B", "C"] {
        let insert = Insert::single_into(api.render_table_name("Post")).value("content", *content);
        api.query(insert.into());
    }

    assert_eq!(
        3,
        api.query(Select::from_table(api.render_table_name("Post")).into())
            .len()
    );

    let dm2 = r#"
        model Post {
            id        BigInt         @id
            content   String?
            createdAt DateTime    @default(now())
            published Boolean     @default(false)
            title     String      @default("")
            updatedAt DateTime    @default(now())
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();

    api.assert_schema().assert_table("Post", |model| {
        model.assert_pk(|pk| pk.assert_columns(&["id"]).assert_has_no_autoincrement())
    });

    // Check that the migration is idempotent.
    api.schema_push_w_datasource(dm2)
        .migration_id(Some("idempotency-check"))
        .send()
        .assert_green()
        .assert_no_steps();

    assert_eq!(
        3,
        api.query(Select::from_table(api.render_table_name("Post")).into())
            .len()
    );
}

#[test_connector(tags(CockroachDb))]
fn on_delete_referential_actions_should_work(api: TestApi) {
    let actions = &[
        (ReferentialAction::SetNull, ForeignKeyAction::SetNull),
        (ReferentialAction::Cascade, ForeignKeyAction::Cascade),
        (ReferentialAction::NoAction, ForeignKeyAction::NoAction),
    ];

    for (ra, fka) in actions {
        let dm = format!(
            r#"
            model A {{
                id BigInt @id @default(autoincrement())
                b      B[]
            }}

            model B {{
                id   BigInt @id
                aId  BigInt?
                a    A?    @relation(fields: [aId], references: [id], onDelete: {ra})
            }}
        "#
        );

        api.schema_push_w_datasource(&dm).send().assert_green();

        api.assert_schema().assert_table("B", |table| {
            table.assert_foreign_keys_count(1).assert_fk_on_columns(&["aId"], |fk| {
                fk.assert_references("A", &["id"])
                    .assert_referential_action_on_delete(*fka)
            })
        });

        api.schema_push_w_datasource("").send().assert_green();
    }
}

#[test_connector(tags(CockroachDb))]
fn changing_all_referenced_columns_of_foreign_key_works(api: TestApi) {
    let dm1 = r#"
       model Post {
          id        BigInt     @default(autoincrement()) @id
          author    User?      @relation(fields: [authorId], references: [id])
          authorId  BigInt?
        }

        model User {
          id       BigInt     @default(autoincrement()) @id
          posts    Post[]
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();

    let dm2 = r#"
        model Post {
          id        BigInt     @default(autoincrement()) @id
          author    User?      @relation(fields: [authorId], references: [uid])
          authorId  BigInt?
        }

        model User {
          uid   BigInt    @id
          posts Post[]
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green();
}

#[test_connector(tags(CockroachDb))]
fn unique_constraint_errors_in_migrations_must_return_a_known_error(api: TestApi) {
    let dm = r#"
        model Fruit {
            id   BigInt @id @default(autoincrement())
            name String
        }
    "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    let insert = Insert::multi_into(api.render_table_name("Fruit"), ["name"])
        .values(("banana",))
        .values(("apple",))
        .values(("banana",));

    api.query(insert.into());

    let dm2 = r#"
        model Fruit {
            id   BigInt @id @default(autoincrement())
            name String @unique
        }
    "#;

    let res = api
        .schema_push_w_datasource(dm2)
        .force(true)
        .migration_id(Some("the-migration"))
        .send_unwrap_err()
        .to_user_facing();

    let json_error = serde_json::to_value(&res).unwrap();

    let expected_msg = if api.is_vitess() {
        "Unique constraint failed on the (not available)"
    } else if api.is_mysql() || api.is_mssql() {
        "Unique constraint failed on the constraint: `Fruit_name_key`"
    } else {
        "Unique constraint failed on the fields: (`name`)"
    };

    let expected_target = if api.is_vitess() {
        serde_json::Value::Null
    } else if api.is_mysql() || api.is_mssql() {
        json!("Fruit_name_key")
    } else {
        json!(["name"])
    };

    let expected_json = json!({
        "is_panic": false,
        "message": expected_msg,
        "meta": {
            "target": expected_target,
        },
        "error_code": "P2002",
    });

    assert_eq!(json_error, expected_json);
}

#[test_connector(tags(CockroachDb))]
fn created_at_does_not_get_arbitrarily_migrated(api: TestApi) {
    use quaint::ast::Insert;

    let dm1 = r#"
        model Fruit {
            id BigInt @id @default(autoincrement())
            name String
            createdAt DateTime @default(now())
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.assert_schema().assert_table("Fruit", |t| {
        t.assert_column("createdAt", |c| {
            c.assert_default_kind(Some(sql_schema_describer::DefaultKind::Now))
        })
    });

    let insert = Insert::single_into(api.render_table_name("Fruit")).value("name", "banana");
    api.query(insert.into());

    api.schema_push_w_datasource(dm1)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(CockroachDb))]
fn sequences_without_options_can_be_created(api: TestApi) {
    let dm = r#"
        datasource test {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            Id Int @id @default(sequence())
        }
    "#;

    api.schema_push(dm).send().assert_green();
    api.schema_push(dm).send().assert_green().assert_no_steps();

    let sql = expect![[r#"
        -- CreateTable
        CREATE TABLE "Test" (
            "Id" INT4 NOT NULL GENERATED BY DEFAULT AS IDENTITY,

            CONSTRAINT "Test_pkey" PRIMARY KEY ("Id")
        );
    "#]];
    api.expect_sql_for_schema(dm, &sql);
}

#[test_connector(tags(CockroachDb))]
fn sequences_with_options_can_be_created(api: TestApi) {
    let dm = r#"
        datasource test {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            Id Int @id @default(sequence(minValue: 10, maxValue: 39, cache: 4, increment: 3, start: 12))
        }
    "#;

    api.schema_push(dm).send().assert_green();
    api.schema_push(dm).send().assert_green().assert_no_steps();

    let sql = expect![[r#"
        -- CreateTable
        CREATE TABLE "Test" (
            "Id" INT4 NOT NULL GENERATED BY DEFAULT AS IDENTITY (INCREMENT 3 CACHE 4 START 12 MINVALUE 10 MAXVALUE 39),

            CONSTRAINT "Test_pkey" PRIMARY KEY ("Id")
        );
    "#]];
    api.expect_sql_for_schema(dm, &sql);
}

#[test_connector(tags(CockroachDb))]
fn sequences_without_options_can_be_created_on_non_id_fields(api: TestApi) {
    let dm = r#"
        datasource test {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            id BigInt @id @default(autoincrement())
            seqCol Int @default(sequence())
        }
    "#;

    api.schema_push(dm).send().assert_green();
    api.schema_push(dm).send().assert_green().assert_no_steps();

    let sql = expect![[r#"
        -- CreateTable
        CREATE TABLE "Test" (
            "id" INT8 NOT NULL DEFAULT unique_rowid(),
            "seqCol" INT4 NOT NULL GENERATED BY DEFAULT AS IDENTITY,

            CONSTRAINT "Test_pkey" PRIMARY KEY ("id")
        );
    "#]];
    api.expect_sql_for_schema(dm, &sql);
}

#[test_connector(tags(CockroachDb))]
fn autoincrement_is_idempotent(api: TestApi) {
    // https://github.com/prisma/prisma/issues/12244

    let dm1 = r#"
        model order {
          orderId BigInt @id @default(autoincrement())
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.schema_push_w_datasource(dm1).send().assert_no_steps();
}

#[test_connector(tags(CockroachDb))]
fn alter_sequence_to_default(api: TestApi) {
    let schema1 = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            Id Int @id @default(sequence(minValue: 10, maxValue: 39, cache: 4, increment: 3, start: 12))
        }
    "#;

    let schema2 = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            Id Int @id @default(sequence())
        }
    "#;

    api.schema_push(schema1)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push(schema1).send().assert_green().assert_no_steps();

    api.schema_push(schema2)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push(schema2).send().assert_green().assert_no_steps();
}

#[test_connector(tags(CockroachDb))]
fn alter_sequence(api: TestApi) {
    let schema1 = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            Id Int @id @default(sequence(minValue: 10, maxValue: 39, cache: 4, increment: 3, start: 12))
        }
    "#;

    let schema2 = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Test {
            Id Int @id @default(sequence(minValue: 8, maxValue: 9009, cache: 12, increment: 33, start: 9))
        }
    "#;

    api.schema_push(schema1)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push(schema1).send().assert_green().assert_no_steps();

    api.schema_push(schema2)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push(schema2).send().assert_green().assert_no_steps();
}

// https://github.com/prisma/prisma/issues/13842
#[test_connector(tags(CockroachDb))]
fn mapped_enum_defaults_must_work(api: TestApi) {
    let schema = r#"
        datasource db {
            provider = "cockroachdb"
            url = "postgres://meowmeowmeow"
        }

        enum Color {
            Red @map("0")
            Green @map("Grün")
            Blue @map("Blu")
            Annoyed @map("pfuh 🙄...")
        }

        model Test {
            id Int @id
            mainColor Color @default(Green)
            secondaryColor Color @default(Red)
            colorOrdering Color[] @default([Blue, Red, Green, Red, Blue, Annoyed, Red])
        }
    "#;

    let expect = expect![[r#"
        -- CreateEnum
        CREATE TYPE "Color" AS ENUM ('0', 'Grün', 'Blu', 'pfuh 🙄...');

        -- CreateTable
        CREATE TABLE "Test" (
            "id" INT4 NOT NULL,
            "mainColor" "Color" NOT NULL DEFAULT 'Grün',
            "secondaryColor" "Color" NOT NULL DEFAULT '0',
            "colorOrdering" "Color"[] DEFAULT ARRAY['Blu', '0', 'Grün', '0', 'Blu', 'pfuh 🙄...', '0']::"Color"[],

            CONSTRAINT "Test_pkey" PRIMARY KEY ("id")
        );
    "#]];
    api.expect_sql_for_schema(schema, &expect);

    api.schema_push(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push(schema).send().assert_green().assert_no_steps();
}

// https://github.com/prisma/prisma/issues/12095
#[test_connector(tags(CockroachDb))]
fn json_defaults_with_escaped_quotes_work(api: TestApi) {
    let schema = r#"
        datasource db {
          provider = "cockroachdb"
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
            "id" INT4 NOT NULL,
            "bar" JSONB DEFAULT '{"message": "This message includes a quote: Here''''s it!"}',

            CONSTRAINT "Foo_pkey" PRIMARY KEY ("id")
        );
    "#]];

    api.expect_sql_for_schema(schema, &sql);
}

#[test_connector(tags(CockroachDb))]
fn sequence_with_multiple_models_works(api: TestApi) {
    let schema = r#"
        datasource db {
          provider = "cockroachdb"
          url      = env("DATABASE_URL")
        }

        model TestModel {
            id BigInt @id @default(autoincrement())
        }

        model TestModelSeq {
            name String
            id Int @id @default(sequence())
        }
    "#;

    let sql = expect![[r#"
        -- CreateTable
        CREATE TABLE "TestModel" (
            "id" INT8 NOT NULL DEFAULT unique_rowid(),

            CONSTRAINT "TestModel_pkey" PRIMARY KEY ("id")
        );

        -- CreateTable
        CREATE TABLE "TestModelSeq" (
            "name" STRING NOT NULL,
            "id" INT4 NOT NULL GENERATED BY DEFAULT AS IDENTITY,

            CONSTRAINT "TestModelSeq_pkey" PRIMARY KEY ("id")
        );
    "#]];

    api.expect_sql_for_schema(schema, &sql);

    api.schema_push(schema).send().assert_green();
    api.schema_push(schema).send().assert_green().assert_no_steps();
}

#[test_connector(tags(CockroachDb))]
fn bigint_defaults_work(api: TestApi) {
    let schema = r#"
        datasource mypg {
            provider = "cockroachdb"
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
            "id" STRING NOT NULL,
            "bar" INT8 NOT NULL DEFAULT 0,

            CONSTRAINT "foo_pkey" PRIMARY KEY ("id")
        );
    "#]];
    api.expect_sql_for_schema(schema, &sql);

    api.schema_push(schema).send().assert_green();
    api.schema_push(schema).send().assert_green().assert_no_steps();
}

// regression test for https://github.com/prisma/prisma/issues/20557
#[test_connector(tags(CockroachDb), exclude(CockroachDb231))]
fn alter_type_works(api: TestApi) {
    let schema = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model test {
            id Int @id
            one BigInt
            two BigInt
        }

    "#;
    api.schema_push(schema).send().assert_green();
    api.schema_push(schema).send().assert_green().assert_no_steps();

    let to_schema = r#"
        datasource db {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model test {
            id Int @id
            one Int
            two Int
        }

    "#;

    let migration = api.connector_diff(
        DiffTarget::Datamodel(vec![("schema.prisma".to_string(), schema.into())]),
        DiffTarget::Datamodel(vec![("schema.prisma".to_string(), to_schema.into())]),
        None,
    );

    // panic!("{migration}");
    api.raw_cmd(&migration);
}

#[test_connector(tags(CockroachDb))]
fn schema_from_introspection_docs_works(api: TestApi) {
    let sql = r#"
        CREATE TABLE "User" (
          id INT8 PRIMARY KEY DEFAULT unique_rowid(),
          name STRING(255),
          email STRING(255) UNIQUE NOT NULL
        );

        CREATE TABLE "Post" (
          id INT8 PRIMARY KEY DEFAULT unique_rowid(),
          title STRING(255) UNIQUE NOT NULL,
          "createdAt" TIMESTAMP NOT NULL DEFAULT now(),
          content STRING,
          published BOOLEAN NOT NULL DEFAULT false,
          "authorId" INT8 NOT NULL,
          FOREIGN KEY ("authorId") REFERENCES "User"(id)
        );

        CREATE TABLE "Profile" (
          id INT8 PRIMARY KEY DEFAULT unique_rowid(),
          bio STRING,
          "userId" INT8 UNIQUE NOT NULL,
          FOREIGN KEY ("userId") REFERENCES "User"(id)
        );
    "#;
    let introspected_schema = r#"
        datasource crdb {
            provider = "cockroachdb"
            url = env("TEST_DATABASE_URL")
        }

        model Post {
          id        BigInt   @id @default(autoincrement())
          title     String   @unique @crdb.String(255)
          createdAt DateTime @default(now()) @crdb.Timestamp(6)
          content   String?
          published Boolean  @default(false)
          authorId  BigInt
          User      User     @relation(fields: [authorId], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model Profile {
          id     BigInt  @id @default(autoincrement())
          bio    String?
          userId BigInt  @unique
          User   User    @relation(fields: [userId], references: [id], onDelete: NoAction, onUpdate: NoAction)
        }

        model User {
          id      BigInt   @id @default(autoincrement())
          name    String?  @crdb.String(255)
          email   String   @unique @crdb.String(255)
          Post    Post[]
          Profile Profile?
        }
    "#;

    api.raw_cmd(sql);

    let migration = api.connector_diff(
        DiffTarget::Database,
        DiffTarget::Datamodel(vec![(
            "schema.prisma".to_string(),
            SourceFile::new_static(introspected_schema),
        )]),
        None,
    );

    let expected = expect!["-- This is an empty migration."];
    expected.assert_eq(&migration);
}

#[test]
fn cockroach_introspection_with_postgres_provider_fails() {
    let test_db = test_setup::only!(CockroachDb);
    let (_, url_str) = tok(test_setup::postgres::create_postgres_database(
        test_db.url(),
        "cockroach_introspection_with_postgres_provider_fails",
    ))
    .unwrap();

    let me = schema_core::schema_api(None, None).unwrap();

    tok(me.db_execute(DbExecuteParams {
        datasource_type: DbExecuteDatasourceType::Url(UrlContainer { url: url_str.clone() }),
        script: r#"
            CREATE TABLE "prisma-tests"."Post" (
                "id" TEXT NOT NULL,
                "title" VARCHAR NOT NULL,
                "content" STRING,
                "authorId" CHARACTER VARYING,
                "exampleChar" CHAR,
                "exampleCharLength" CHAR(16),
                "exampleBit" BIT,
                "exampleBitLength" BIT(16),
                PRIMARY KEY ("id")
            );

            CREATE TABLE "prisma-tests"."User" (
                "id" TEXT,
                "email" STRING(32) NOT NULL,
                "name" CHARACTER VARYING(32),
                PRIMARY KEY ("id")
            );
            "#
        .to_owned(),
    }))
    .unwrap();

    let schema = format! {
        r#"
            datasource db {{
                provider = "postgres"
                url = "{url_str}"
            }}
        "#,
    };

    let error = tok(me.introspect(schema_core::json_rpc::types::IntrospectParams {
        composite_type_depth: -1,
        force: false,
        schema: SchemasContainer {
            files: vec![SchemaContainer {
                path: "schema.prisma".to_string(),
                content: schema,
            }],
        },
        base_directory_path: "/".to_string(),
        namespaces: None,
    }))
    .unwrap_err()
    .message()
    .unwrap()
    .to_owned();

    let expected_err = expect![
        r#"You are trying to connect to a CockroachDB database, but the provider in your Prisma schema is `postgresql`. Please change it to `cockroachdb`."#
    ];

    expected_err.assert_eq(&error);
}
