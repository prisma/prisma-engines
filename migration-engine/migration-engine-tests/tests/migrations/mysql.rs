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

    api.schema_push(schema).send_sync();

    let sql_schema = api
        .assert_schema()
        .assert_table_bang("Human", |table| {
            table
                .assert_foreign_keys_count(1)?
                .assert_fk_on_columns(&["catname"], |fk| fk.assert_references("Cat", &["name"]))?
                .assert_indexes_count(1)?
                .assert_index_on_columns(&["catname"], |idx| idx.assert_is_not_unique())
        })
        .into_schema();

    // Test that after introspection, we do not migrate further.
    api.schema_push(schema)
        .force(true)
        .send_sync()
        .assert_green_bang()
        .assert_no_steps();

    api.assert_schema().assert_equals(&sql_schema).unwrap();
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

    api.schema_push(dm1).send_sync().assert_green_bang();
    api.schema_push(dm1).send_sync().assert_green_bang().assert_no_steps();
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

    api.schema_push(schema).send_sync().assert_green_bang();
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

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_required())?
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

    api.schema_push(dm2).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_nullable())?
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

    api.schema_push(dm1).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_required())?
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

    api.schema_push(dm2)
        .force(true)
        .send_sync()
        .assert_executable()
        .assert_has_executed_steps();

    api.assert_schema().assert_table_bang("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_required())?
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
        datasource mysql {
            provider = "mysql"
            url = "mysql://localhost/test"
        }

        model A {
            id Int @id
    "#
    .to_owned();

    for (field_name, prisma_type, native_type, _) in types {
        writeln!(&mut dm, "    {} {} @mysql.{}", field_name, prisma_type, native_type).unwrap();
    }

    dm.push_str("}\n");

    api.schema_push(&dm).send_sync().assert_green_bang();

    api.assert_schema().assert_table_bang("A", |table| {
        types.iter().fold(
            Ok(table),
            |table, (field_name, _prisma_type, _native_type, database_type)| {
                table.and_then(|table| table.assert_column(field_name, |col| col.assert_full_data_type(database_type)))
            },
        )
    });

    // Check that the migration is idempotent
    api.schema_push(dm).send_sync().assert_green_bang().assert_no_steps();
}

#[test_connector(tags(Mysql))]
fn default_current_timestamp_precision_follows_column_precision(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm = format!(
        "
        {}

        model A {{
            id Int @id
            createdAt DateTime @db.DateTime(7) @default(now())
        }}
        ",
        api.datasource_block()
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
