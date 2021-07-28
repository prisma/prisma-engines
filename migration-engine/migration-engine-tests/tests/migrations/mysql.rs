use indoc::indoc;
use migration_engine_tests::sync_test_api::*;
use std::fmt::Write as _;

// We need to test this specifically for mysql, because foreign keys are indexes, and they are
// inferred as both foreign key and index by the sql-schema-describer. We do not want to
// create/delete a second index.
#[test_connector(tags(Mysql))]
fn indexes_on_foreign_key_fields_are_not_created_twice(api: TestApi) {
    let schema = r#"
        model Human {
            id String @id
            catname String
            cat_rel Cat @relation(fields: [catname], references: [name])
        }

        model Cat {
            id String @id
            name String @unique
            humans Human[]
        }
    "#;

    api.schema_push_w_datasource(schema).send();

    let sql_schema = api
        .assert_schema()
        .assert_table("Human", |table| {
            table
                .assert_foreign_keys_count(1)
                .assert_fk_on_columns(&["catname"], |fk| fk.assert_references("Cat", &["name"]))
                .assert_indexes_count(1)
                .assert_index_on_columns(&["catname"], |idx| idx.assert_is_not_unique())
        })
        .into_schema();

    // Test that after introspection, we do not migrate further.
    api.schema_push_w_datasource(schema)
        .force(true)
        .send()
        .assert_green_bang()
        .assert_no_steps();

    api.assert_schema().assert_equals(&sql_schema);
}

// We have to test this because one enum on MySQL can map to multiple enums in the database.
#[test_connector(tags(Mysql))]
fn enum_creation_is_idempotent(api: TestApi) {
    let dm1 = r#"
        model Cat {
            id String @id
            mood Mood
        }

        model Human {
            id String @id
            mood Mood
        }

        enum Mood {
            HAPPY
            HUNGRY
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green_bang();
    api.schema_push_w_datasource(dm1)
        .send()
        .assert_green_bang()
        .assert_no_steps();
}

#[test_connector(tags(Mysql))]
fn enums_work_when_table_name_is_remapped(api: TestApi) {
    let schema = r#"
    model User {
        id         String     @default(uuid()) @id
        status     UserStatus @map("currentStatus___")

        @@map("users")
    }

    enum UserStatus {
        CONFIRMED
        CANCELED
        BLOCKED
    }
    "#;

    api.schema_push_w_datasource(schema).send().assert_green_bang();
}

#[test_connector(tags(Mysql))]
fn arity_of_enum_columns_can_be_changed(api: TestApi) {
    let dm1 = r#"
        enum Color {
            RED
            GREEN
            BLUE
        }

        model A {
            id              Int @id
            primaryColor    Color
            secondaryColor  Color?
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green_bang();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_required())
            .assert_column("secondaryColor", |col| col.assert_is_nullable())
    });

    let dm2 = r#"
        enum Color {
            RED
            GREEN
            BLUE
        }

        model A {
            id              Int @id
            primaryColor    Color?
            secondaryColor  Color
        }
    "#;

    api.schema_push_w_datasource(dm2).send().assert_green_bang();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_nullable())
            .assert_column("secondaryColor", |col| col.assert_is_required())
    });
}

#[test_connector(tags(Mysql))]
fn arity_is_preserved_by_alter_enum(api: TestApi) {
    let dm1 = r#"
        enum Color {
            RED
            GREEN
            BLUE
        }

        model A {
            id              Int @id
            primaryColor    Color
            secondaryColor  Color?
        }
    "#;

    api.schema_push_w_datasource(dm1).send().assert_green_bang();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_required())
            .assert_column("secondaryColor", |col| col.assert_is_nullable())
    });

    let dm2 = r#"
        enum Color {
            ROT
            GRUEN
            BLAU
        }

        model A {
            id              Int @id
            primaryColor    Color
            secondaryColor  Color?
        }
    "#;

    api.schema_push_w_datasource(dm2)
        .force(true)
        .send()
        .assert_executable()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_required())
            .assert_column("secondaryColor", |col| col.assert_is_nullable())
    });
}

#[test_connector(tags(Mysql))]
fn native_type_columns_can_be_created(api: TestApi) {
    let types = &[
        ("int", "Int", "Int", if api.is_mysql_8() { "int" } else { "int(11)" }),
        (
            "smallint",
            "Int",
            "SmallInt",
            if api.is_mysql_8() { "smallint" } else { "smallint(6)" },
        ),
        ("tinyint", "Boolean", "TinyInt", "tinyint(1)"),
        (
            "tinyintInt",
            "Int",
            "TinyInt",
            if api.is_mysql_8() { "tinyint" } else { "tinyint(4)" },
        ),
        (
            "mediumint",
            "Int",
            "MediumInt",
            if api.is_mysql_8() { "mediumint" } else { "mediumint(9)" },
        ),
        (
            "bigint",
            "BigInt",
            "BigInt",
            if api.is_mysql_8() { "bigint" } else { "bigint(20)" },
        ),
        ("decimal", "Decimal", "Decimal(5, 3)", "decimal(5,3)"),
        ("float", "Float", "Float", "float"),
        ("double", "Float", "Double", "double"),
        ("bits", "Bytes", "Bit(10)", "bit(10)"),
        ("bit", "Boolean", "Bit(1)", "bit(1)"),
        ("chars", "String", "Char(10)", "char(10)"),
        ("varchars", "String", "VarChar(500)", "varchar(500)"),
        ("binary", "Bytes", "Binary(230)", "binary(230)"),
        ("varbinary", "Bytes", "VarBinary(150)", "varbinary(150)"),
        ("tinyBlob", "Bytes", "TinyBlob", "tinyblob"),
        ("blob", "Bytes", "Blob", "blob"),
        ("mediumBlob", "Bytes", "MediumBlob", "mediumblob"),
        ("longBlob", "Bytes", "LongBlob", "longblob"),
        ("tinytext", "String", "TinyText", "tinytext"),
        ("text", "String", "Text", "text"),
        ("mediumText", "String", "MediumText", "mediumtext"),
        ("longText", "String", "LongText", "longtext"),
        ("date", "DateTime", "Date", "date"),
        ("timeWithPrecision", "DateTime", "Time(3)", "time(3)"),
        ("dateTimeWithPrecision", "DateTime", "DateTime(3)", "datetime(3)"),
        (
            "timestampWithPrecision",
            "DateTime @default(now())",
            "Timestamp(3)",
            "timestamp(3)",
        ),
        ("year", "Int", "Year", if api.is_mysql_8() { "year" } else { "year(4)" }),
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

    api.schema_push_w_datasource(&dm).send().assert_green_bang();

    api.assert_schema().assert_table("A", |table| {
        types.iter().fold(
            table,
            |table, (field_name, _prisma_type, _native_type, database_type)| {
                table.assert_column(field_name, |col| col.assert_full_data_type(database_type))
            },
        )
    });

    // Check that the migration is idempotent
    api.schema_push_w_datasource(dm)
        .send()
        .assert_green_bang()
        .assert_no_steps();
}

#[test_connector(tags(Mysql))]
fn default_current_timestamp_precision_follows_column_precision(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm = api.datamodel_with_provider(
        "
        model A {
            id Int @id
            createdAt DateTime @db.DateTime(7) @default(now())
        }
        ",
    );

    let expected_migration = indoc!(
        r#"
        -- CreateTable
        CREATE TABLE `A` (
            `id` INTEGER NOT NULL,
            `createdAt` DATETIME(7) NOT NULL DEFAULT CURRENT_TIMESTAMP(7),

            PRIMARY KEY (`id`)
        ) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
        "#
    );

    api.create_migration("01init", &dm, &migrations_directory)
        .send_sync()
        .assert_migration("01init", |migration| migration.assert_contents(expected_migration));
}

#[test_connector(tags(Mysql))]
fn mysql_apply_migrations_errors_gives_the_failed_sql(api: TestApi) {
    let dm = "";
    let migrations_directory = api.create_migrations_directory();

    let migration = r#"
        CREATE TABLE `Cat` ( id INTEGER PRIMARY KEY );

        DROP TABLE `Emu`;

        CREATE TABLE `Emu` (
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

    let expectation = if api.is_vitess() {
        expect![[r#"
            A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

            Migration name: <migration-name>

            Database error code: 1051

            Database error:
            target: test.0.master: vttablet: rpc error: code = InvalidArgument desc = Unknown table 'vt_test_0.Emu' (errno 1051) (sqlstate 42S02) (CallerID: userData1): Sql: "drop table Emu", BindVars: {}

            Please check the query number 2 from the migration file.

        "#]]
    } else if cfg!(windows) {
        expect![[r#"
            A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

            Migration name: <migration-name>

            Database error code: 1051

            Database error:
            Unknown table 'mysql_apply_migrations_errors_gives_the_failed_sql.emu'

            Please check the query number 2 from the migration file.

        "#]]
    } else {
        expect![[r#"
            A migration failed to apply. New migrations cannot be applied before the error is recovered from. Read more about how to resolve migration issues in a production database: https://pris.ly/d/migrate-resolve

            Migration name: <migration-name>

            Database error code: 1051

            Database error:
            Unknown table 'mysql_apply_migrations_errors_gives_the_failed_sql.Emu'

            Please check the query number 2 from the migration file.

        "#]]
    };

    let first_segment = err
        .split_terminator("DbError {")
        .next()
        .unwrap()
        .split_terminator("   0: ")
        .next()
        .unwrap();

    expectation.assert_eq(first_segment)
}
