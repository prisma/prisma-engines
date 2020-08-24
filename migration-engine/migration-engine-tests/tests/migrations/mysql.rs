use migration_engine_tests::sql::*;
use std::fmt::Write as _;

/// We need to test this specifically for mysql, because foreign keys are indexes, and they are
/// inferred as both foreign key and index by the sql-schema-describer. We do not want to
/// create/delete a second index.
#[test_each_connector(tags("mysql"))]
async fn indexes_on_foreign_key_fields_are_not_created_twice(api: &TestApi) -> TestResult {
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

    api.infer_apply(schema).send().await?;

    let sql_schema = api
        .assert_schema()
        .await?
        .assert_table("Human", |table| {
            table
                .assert_foreign_keys_count(1)?
                .assert_fk_on_columns(&["catname"], |fk| fk.assert_references("Cat", &["name"]))?
                .assert_indexes_count(1)?
                .assert_index_on_columns(&["catname"], |idx| idx.assert_is_not_unique())
        })?
        .into_schema();

    // Test that after introspection, we do not migrate further.
    api.infer_apply(schema)
        .force(Some(true))
        .send()
        .await?
        .assert_green()?
        .assert_no_steps()?;

    api.assert_schema().await?.assert_equals(&sql_schema)?;

    Ok(())
}

// We have to test this because one enum on MySQL can map to multiple enums in the database.
#[test_each_connector(tags("mysql"))]
async fn enum_creation_is_idempotent(api: &TestApi) -> TestResult {
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

    api.infer_apply(dm1).send().await?.assert_green()?;

    api.infer_apply(dm1).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn enums_work_when_table_name_is_remapped(api: &TestApi) -> TestResult {
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

    api.infer_apply(schema).send().await?.assert_green()?;

    Ok(())
}

#[test_each_connector(tags("mysql"), log = "debug,sql_schema_describer=info")]
async fn arity_of_enum_columns_can_be_changed(api: &TestApi) -> TestResult {
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

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_required())?
            .assert_column("secondaryColor", |col| col.assert_is_nullable())
    })?;

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

    api.schema_push(dm2).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_nullable())?
            .assert_column("secondaryColor", |col| col.assert_is_required())
    })?;

    Ok(())
}

#[test_each_connector(tags("mysql"), log = "debug,sql_schema_describer=info")]
async fn arity_is_preserved_by_alter_enum(api: &TestApi) -> TestResult {
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

    api.schema_push(dm1).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_required())?
            .assert_column("secondaryColor", |col| col.assert_is_nullable())
    })?;

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
        .send()
        .await?
        .assert_executable()?
        .assert_has_executed_steps()?;

    api.assert_schema().await?.assert_table("A", |table| {
        table
            .assert_column("primaryColor", |col| col.assert_is_required())?
            .assert_column("secondaryColor", |col| col.assert_is_nullable())
    })?;

    Ok(())
}

#[test_each_connector(tags("mysql"))]
async fn native_type_columns_can_be_created(api: &TestApi) -> TestResult {
    let types = &[
        ("int", "Int", "Int", if api.is_mysql_8() { "int" } else { "int(11)" }),
        (
            "smallint",
            "Int",
            "SmallInt",
            if api.is_mysql_8() { "smallint" } else { "smallint(6)" },
        ),
        (
            "tinyint",
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
            "Int",
            "BigInt",
            if api.is_mysql_8() { "bigint" } else { "bigint(20)" },
        ),
        ("decimal", "Decimal", "Decimal(5, 3)", "decimal(5,3)"),
        ("numeric", "Decimal", "Numeric(4,1)", "decimal(4,1)"),
        ("float", "Float", "Float", "float"),
        ("double", "Float", "Double", "double"),
        ("bits", "Bytes", "Bit(10)", "bit(10)"),
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
        ("dateTimeWithPrecision", "DateTime", "Datetime(3)", "datetime(3)"),
        ("timestampWithPrecision", "DateTime", "Timestamp(3)", "timestamp(3)"),
        ("year", "Int", "Year", if api.is_mysql_8() { "year" } else { "year(4)" }),
    ];

    let mut dm = r#"
        datasource mysql {
            provider = "mysql"
            url = "mysql://localhost/test"
            previewFeatures = ["nativeTypes"]
        }

        model A {
            id Int @id
    "#
    .to_owned();

    for (field_name, prisma_type, native_type, _) in types {
        writeln!(&mut dm, "    {} {} @mysql.{}", field_name, prisma_type, native_type)?;
    }

    dm.push_str("}\n");

    api.schema_push(dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("A", |table| {
        types.iter().fold(
            Ok(table),
            |table, (field_name, _prisma_type, _native_type, database_type)| {
                table.and_then(|table| table.assert_column(field_name, |col| col.assert_full_data_type(database_type)))
            },
        )
    })?;

    Ok(())
}
