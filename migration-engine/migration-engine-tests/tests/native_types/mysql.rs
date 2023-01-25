use migration_engine_tests::test_api::*;
use std::{borrow::Cow, fmt::Write};

/// (source native type, test value to insert, target native type)
type Case = (&'static str, quaint::Value<'static>, &'static [&'static str]);
type Cases = &'static [Case];

const SAFE_CASTS: Cases = &[
    (
        "BigInt",
        quaint::Value::Int64(Some(99999999432)),
        &[
            "Binary(200)",
            "Bit(54)",
            "Blob",
            "Char(20)",
            "Decimal(21,1)",
            "Double",
            "Float",
            "LongBlob",
            "LongText",
            "MediumBlob",
            "MediumText",
            "Text",
            "TinyBlob",
            "TinyText",
            "VarChar(20)",
            "VarBinary(15)",
        ],
    ),
    (
        "Binary(8)",
        quaint::Value::Bytes(Some(Cow::Borrowed(b"08088044"))),
        &[
            "Bit(64)",
            "Blob",
            "Char(64)",
            "Decimal(10,0)",
            "Double",
            "LongBlob",
            "LongText",
            "MediumBlob",
            "MediumInt",
            "MediumText",
            "Text",
            "TinyBlob",
            "TinyText",
            "VarBinary(15)",
            "VarChar(20)",
        ],
    ),
    (
        "Int",
        quaint::Value::Int32(Some(i32::MIN)),
        &[
            "BigInt",
            "Char(20)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "VarChar(20)",
        ],
    ),
    (
        "Bit(32)",
        quaint::Value::Bytes(Some(Cow::Borrowed(b""))),
        &[
            "SmallInt",
            "UnsignedSmallInt",
            "TinyInt",
            "UnsignedTinyInt",
            "Int",
            "MediumInt",
            "TinyText",
            "MediumText",
            "LongText",
            "Text",
            "TinyBlob",
            "MediumBlob",
            "LongBlob",
            "Blob",
            "VarChar(32)",
            "Year",
        ],
    ),
    (
        "Blob",
        quaint::Value::Bytes(Some(Cow::Borrowed(&[0xff]))),
        &["TinyBlob", "MediumBlob", "LongBlob"],
    ),
    (
        "Char(10)",
        quaint::Value::Text(Some(Cow::Borrowed("1234"))),
        &[
            "Blob",
            "Char(11)",
            "LongBlob",
            "LongText",
            "MediumBlob",
            "MediumText",
            "Text",
            "TinyBlob",
            "TinyText",
            "VarChar(10)",
        ],
    ),
    (
        "Date",
        quaint::Value::Text(Some(Cow::Borrowed("2020-01-12"))),
        &[
            "DateTime(0)",
            "Decimal(10,0)",
            "Float",
            "Double",
            "BigInt",
            "UnsignedInt",
            "Int",
            // To string
            "Binary(10)",
            "Bit(64)",
            "Char(10)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "VarBinary(10)",
            "VarChar(10)",
            "Blob",
        ],
    ),
    (
        "DateTime(0)",
        quaint::Value::Text(Some(Cow::Borrowed("2020-01-08 08:00:00"))),
        &[
            "BigInt",
            "UnsignedBigInt",
            "Time(0)",
            "Timestamp(0)",
            "Date",
            "Blob",
            "VarChar(20)",
        ],
    ),
    (
        "Double",
        quaint::Value::Float(Some(3.20)),
        &[
            "Float",
            "Bit(64)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "Blob",
            // integers
            "UnsignedTinyInt",
            "Decimal(10,5)",
            "TinyInt",
            "Int",
            "Json",
            "UnsignedInt",
            "SmallInt",
            "UnsignedSmallInt",
            "MediumInt",
            "UnsignedMediumInt",
            "Year",
        ],
    ),
    (
        "Float",
        quaint::Value::Float(Some(3.20)),
        &[
            "Double",
            "Bit(32)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "Blob",
            // integers
            "UnsignedTinyInt",
            "Decimal(10,5)",
            "TinyInt",
            "Int",
            "Json",
            "UnsignedInt",
            "SmallInt",
            "UnsignedSmallInt",
            "MediumInt",
            "UnsignedMediumInt",
            "Year",
            // Time
            "Time(0)",
        ],
    ),
    (
        "Json",
        quaint::Value::Text(Some(Cow::Borrowed("{\"a\":\"b\"}"))),
        &[
            // To string
            "Binary(10)",
            "Char(10)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "VarBinary(10)",
            "VarChar(10)",
        ],
    ),
    (
        "LongBlob",
        quaint::Value::Bytes(Some(Cow::Borrowed(&[0xff]))),
        &["TinyBlob", "Blob", "MediumBlob"],
    ),
    (
        "MediumBlob",
        quaint::Value::Bytes(Some(Cow::Borrowed(&[0xff]))),
        &["TinyBlob", "Blob", "LongBlob"],
    ),
    (
        "TinyBlob",
        quaint::Value::Bytes(Some(Cow::Borrowed(&[0xff]))),
        &["LongBlob", "Blob", "MediumBlob"],
    ),
    (
        "Time",
        quaint::Value::Int32(Some(20)),
        &[
            "VarChar(20)",
            "BigInt",
            "Int",
            "UnsignedSmallInt",
            "TinyInt",
            "Decimal(20,5)",
        ],
    ),
    (
        "Year",
        quaint::Value::Int32(Some(2000)),
        &[
            // To string
            "Binary(10)",
            "Bit(64)",
            "Char(10)",
            "LongText",
            "LongBlob",
            "TinyBlob",
            "MediumBlob",
            "Blob",
            "MediumText",
            "Text",
            "TinyText",
            "VarBinary(10)",
            "VarChar(10)",
            // To integers
            "Bit(64)",
            "Int",
            "MediumInt",
            "SmallInt",
            "UnsignedBigInt",
            "UnsignedInt",
            "UnsignedMediumInt",
            "UnsignedSmallInt",
            "Float",
            "Double",
        ],
    ),
];

const RISKY_CASTS: Cases = &[
    (
        "BigInt",
        quaint::Value::Int64(Some(100)),
        &[
            "Int",
            "MediumInt",
            "SmallInt",
            "TinyInt",
            "UnsignedBigInt",
            "UnsignedInt",
            "UnsignedMediumInt",
            "UnsignedSmallInt",
            "UnsignedTinyInt",
        ],
    ),
    ("BigInt", quaint::Value::Int64(Some(2000)), &["Year"]),
    (
        "Binary(8)",
        quaint::Value::Bytes(Some(Cow::Borrowed(b"08088044"))),
        &["Bit(32)", "Int", "UnsignedBigInt", "UnsignedInt", "UnsignedMediumInt"],
    ),
    (
        "Binary(1)",
        quaint::Value::Bytes(Some(Cow::Borrowed(b"0"))),
        &["Time(0)", "SmallInt", "TinyInt", "UnsignedSmallInt", "UnsignedTinyInt"],
    ),
    (
        "Binary(4)",
        quaint::Value::Bytes(Some(Cow::Borrowed(b"2000"))),
        &["Year"],
    ),
    (
        "Bit(32)",
        quaint::Value::Bytes(Some(Cow::Borrowed(b""))),
        &["Decimal(10,2)", "Double", "Float"],
    ),
    (
        "Blob",
        quaint::Value::Bytes(Some(Cow::Borrowed(b"abc"))),
        &[
            "Binary(10)",
            "Char(10)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "VarBinary(5)",
            "VarChar(20)",
        ],
    ),
    (
        "Decimal(20,5)",
        quaint::Value::Text(Some(Cow::Borrowed("350"))),
        &["BigInt", "UnsignedBigInt", "Time(0)", "Json"],
    ),
    (
        "Double",
        quaint::Value::Float(Some(0f32)),
        &["Char(40)", "VarBinary(40)", "VarChar(40)"],
    ),
    (
        "Float",
        quaint::Value::Float(Some(0f32)),
        &["Char(40)", "VarBinary(40)", "VarChar(40)"],
    ),
    (
        "LongBlob",
        quaint::Value::Bytes(Some(Cow::Borrowed(b"abc"))),
        &[
            "Binary(10)",
            "Char(10)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "VarBinary(5)",
            "VarChar(20)",
        ],
    ),
    (
        "MediumBlob",
        quaint::Value::Bytes(Some(Cow::Borrowed(b"abc"))),
        &[
            "Binary(10)",
            "Char(10)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "VarBinary(5)",
            "VarChar(20)",
        ],
    ),
    ("SmallInt", quaint::Value::Int32(Some(1990)), &["Year", "Double"]),
    (
        "TinyBlob",
        quaint::Value::Bytes(Some(Cow::Borrowed(b"abc"))),
        &[
            "Binary(10)",
            "Char(10)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "VarBinary(5)",
            "VarChar(20)",
        ],
    ),
    (
        "Time(0)",
        quaint::Value::Int32(Some(5002)),
        &["Date", "DateTime(0)", "Timestamp(0)"],
    ),
    (
        "Year",
        quaint::Value::Text(Some(Cow::Borrowed("1999"))),
        &["Decimal(10,0)", "Json"],
    ),
];

const IMPOSSIBLE_CASTS: Cases = &[
    (
        "BigInt",
        quaint::Value::Int64(Some(500)),
        &["Decimal(15,6)", "Date", "DateTime(0)", "Json", "Timestamp(0)"],
    ),
    (
        "Binary(12)",
        quaint::Value::Bytes(Some(Cow::Borrowed(b"8080008"))),
        &["Date", "DateTime(0)", "Json", "Timestamp(0)"],
    ),
    (
        "Bit(32)",
        quaint::Value::Bytes(Some(Cow::Borrowed(b""))),
        &["Date", "DateTime(0)", "Time(0)", "Timestamp(0)", "Json"],
    ),
    (
        "Blob",
        quaint::Value::Bytes(Some(Cow::Borrowed(&[0x00]))),
        &[
            "TinyInt",
            "BigInt",
            "Date",
            "DateTime(0)",
            "Decimal(10,5)",
            "Double",
            "Float",
            "Int",
            "Json",
            "MediumInt",
            "SmallInt",
            "Time(0)",
            "Timestamp(0)",
            "UnsignedInt",
            "UnsignedMediumInt",
            "UnsignedSmallInt",
            "UnsignedTinyInt",
            "UnsignedBigInt",
            "Year",
        ],
    ),
    (
        "Date",
        quaint::Value::Text(Some(Cow::Borrowed("2020-01-12"))),
        &[
            "TinyInt",
            "UnsignedTinyInt",
            "Year",
            "SmallInt",
            "UnsignedSmallInt",
            "UnsignedMediumInt",
            "MediumInt",
        ],
    ),
    (
        "DateTime(0)",
        quaint::Value::Text(Some(Cow::Borrowed("2020-01-08 08:00:00"))),
        &[
            "TinyInt",
            "UnsignedTinyInt",
            "Int",
            "UnsignedInt",
            "SmallInt",
            "UnsignedSmallInt",
            "MediumInt",
            "UnsignedMediumInt",
            "Year",
        ],
    ),
    (
        "Double",
        quaint::Value::Float(Some(3.20)),
        &["Binary(10)", "Date", "Timestamp(0)", "DateTime(0)"],
    ),
    (
        "Float",
        quaint::Value::Float(Some(3.20)),
        &["Binary(10)", "Date", "Timestamp(0)", "DateTime(0)"],
    ),
    (
        "Json",
        quaint::Value::Text(Some(Cow::Borrowed("{\"a\":\"b\"}"))),
        &[
            // Integer types
            "Bit(64)",
            "Int",
            "MediumInt",
            "SmallInt",
            "TinyInt",
            "UnsignedBigInt",
            "UnsignedInt",
            "UnsignedMediumInt",
            "UnsignedSmallInt",
            "UnsignedTinyInt",
            "Float",
            "Double",
        ],
    ),
    (
        "LongBlob",
        quaint::Value::Bytes(Some(Cow::Borrowed(&[0x00]))),
        &[
            "TinyInt",
            "BigInt",
            "Date",
            "DateTime(0)",
            "Decimal(10,5)",
            "Double",
            "Float",
            "Int",
            "Json",
            "MediumInt",
            "SmallInt",
            "Time(0)",
            "Timestamp(0)",
            "UnsignedInt",
            "UnsignedMediumInt",
            "UnsignedSmallInt",
            "UnsignedTinyInt",
            "UnsignedBigInt",
            "Year",
        ],
    ),
    (
        "MediumBlob",
        quaint::Value::Bytes(Some(Cow::Borrowed(&[0x00]))),
        &[
            "TinyInt",
            "BigInt",
            "Date",
            "DateTime(0)",
            "Decimal(10,5)",
            "Double",
            "Float",
            "Int",
            "Json",
            "MediumInt",
            "SmallInt",
            "Time(0)",
            "Timestamp(0)",
            "UnsignedInt",
            "UnsignedMediumInt",
            "UnsignedSmallInt",
            "UnsignedTinyInt",
            "UnsignedBigInt",
            "Year",
        ],
    ),
    ("Time(0)", quaint::Value::Int32(Some(0)), &["Json", "Year"]),
    (
        "TinyBlob",
        quaint::Value::Bytes(Some(Cow::Borrowed(&[0x00]))),
        &[
            "TinyInt",
            "BigInt",
            "Date",
            "DateTime(0)",
            "Decimal(10,5)",
            "Double",
            "Float",
            "Int",
            "Json",
            "MediumInt",
            "SmallInt",
            "Time(0)",
            "Timestamp(0)",
            "UnsignedInt",
            "UnsignedMediumInt",
            "UnsignedSmallInt",
            "UnsignedTinyInt",
            "UnsignedBigInt",
            "Year",
        ],
    ),
    (
        "Year",
        quaint::Value::Int32(Some(2001)),
        &[
            "TinyInt",
            "UnsignedTinyInt",
            "Date",
            "Time(0)",
            "DateTime(0)",
            "Timestamp(0)",
        ],
    ),
];

fn native_type_name_to_prisma_scalar_type_name(scalar_type: &str) -> &'static str {
    /// Map from native type name to prisma scalar type name.
    const TYPES_MAP: &[(&str, &str)] = &[
        ("BigInt", "BigInt"),
        ("Binary", "Bytes"),
        ("Bit", "Bytes"),
        ("Blob", "Bytes"),
        ("Char", "String"),
        ("Date", "DateTime"),
        ("DateTime", "DateTime"),
        ("Decimal", "Decimal"),
        ("Double", "Float"),
        ("Float", "Float"),
        ("Int", "Int"),
        ("Json", "Json"),
        ("LongBlob", "Bytes"),
        ("LongText", "String"),
        ("MediumBlob", "Bytes"),
        ("MediumInt", "Int"),
        ("MediumText", "String"),
        ("SmallInt", "Int"),
        ("Text", "String"),
        ("Time", "DateTime"),
        ("Timestamp", "DateTime"),
        ("TinyBlob", "Bytes"),
        ("TinyInt", "Int"),
        ("TinyText", "String"),
        ("UnsignedBigInt", "BigInt"),
        ("UnsignedInt", "Int"),
        ("UnsignedMediumInt", "Int"),
        ("UnsignedSmallInt", "Int"),
        ("UnsignedTinyInt", "Int"),
        ("VarBinary", "Bytes"),
        ("VarChar", "String"),
        ("Year", "Int"),
    ];

    let scalar_type =
        scalar_type.trim_end_matches(|ch: char| [' ', ',', '(', ')'].contains(&ch) || ch.is_ascii_digit());

    let idx = TYPES_MAP
        .binary_search_by_key(&scalar_type, |(native, _prisma)| native)
        .map_err(|_err| format!("Could not find {} in TYPES_MAP", scalar_type))
        .unwrap();

    TYPES_MAP[idx].1
}

fn colnames_for_cases(cases: Cases) -> Vec<String> {
    let max_colname = cases.iter().map(|(_, _, to_types)| to_types.len()).max().unwrap();

    std::iter::repeat(())
        .enumerate()
        .take(max_colname)
        .map(|(idx, _)| format!("col{}", idx))
        .collect()
}

fn expand_cases<'a, 'b>(
    from_type: &str,
    test_value: &'a quaint::Value,
    (to_types, nullable): (&[&str], bool),
    dm1: &'b mut String,
    dm2: &'b mut String,
    colnames: &'a [String],
) -> quaint::ast::SingleRowInsert<'a> {
    let mut insert = quaint::ast::Insert::single_into("Test");

    for dm in std::iter::once(&mut *dm1).chain(std::iter::once(&mut *dm2)) {
        dm.clear();
        dm.push_str("model Test {\nid Int @id @default(autoincrement())\n");
    }

    for (idx, _) in std::iter::repeat(()).enumerate().take(to_types.len()) {
        writeln!(
            dm1,
            "{colname} {scalar_type}{nullability} @db.{native_type}",
            colname = colnames[idx],
            scalar_type = native_type_name_to_prisma_scalar_type_name(from_type),
            native_type = from_type,
            nullability = if nullable { "?" } else { "" },
        )
        .unwrap();
    }

    for (idx, to_type) in to_types.iter().enumerate() {
        writeln!(
            dm2,
            "{colname} {scalar_type}{nullability} @db.{native_type}",
            colname = colnames[idx],
            scalar_type = native_type_name_to_prisma_scalar_type_name(to_type),
            native_type = to_type,
            nullability = if nullable { "?" } else { "" },
        )
        .unwrap();

        insert = insert.value(colnames[idx].as_str(), test_value.clone());
    }

    for dm in std::iter::once(&mut *dm1).chain(std::iter::once(&mut *dm2)) {
        dm.push('}');
    }

    insert
}

fn type_is_unsupported_mariadb(ty: &str) -> bool {
    ty == "Time(0)" || ty == "Json"
}

fn type_is_unsupported_mysql_5_6(ty: &str) -> bool {
    type_is_unsupported_mariadb(ty)
}

fn filter_from_types(api: &TestApi, cases: Cases) -> Cow<'static, [Case]> {
    if api.is_mariadb() {
        return Cow::Owned(
            cases
                .iter()
                .cloned()
                .filter(|(ty, _, _)| !type_is_unsupported_mariadb(ty))
                .collect(),
        );
    }

    if api.is_mysql_5_6() {
        return Cow::Owned(
            cases
                .iter()
                .cloned()
                .filter(|(ty, _, _)| !type_is_unsupported_mysql_5_6(ty))
                .collect(),
        );
    }

    cases.into()
}

fn filter_to_types(api: &TestApi, to_types: &'static [&'static str]) -> Cow<'static, [&'static str]> {
    if api.is_mariadb() {
        return Cow::Owned(
            to_types
                .iter()
                .cloned()
                .filter(|ty| !type_is_unsupported_mariadb(ty))
                .collect(),
        );
    }

    if api.is_mysql_5_6() {
        return Cow::Owned(
            to_types
                .iter()
                .cloned()
                .filter(|ty| !type_is_unsupported_mysql_5_6(ty))
                .collect(),
        );
    }

    to_types.into()
}

#[test_connector(tags(Mysql))]
fn safe_casts_with_existing_data_should_work(api: TestApi) {
    let connector = psl::builtin_connectors::MYSQL;
    let mut dm1 = String::with_capacity(256);
    let mut dm2 = String::with_capacity(256);
    let colnames = colnames_for_cases(SAFE_CASTS);
    let safe_casts = filter_from_types(&api, SAFE_CASTS);

    for (from_type, test_value, to_types) in safe_casts.iter() {
        let span = tracing::info_span!("SafeCasts", from = %from_type, to = ?to_types);
        let _span = span.enter();

        let to_types = filter_to_types(&api, to_types);

        tracing::info!("initial migration");

        let insert = expand_cases(
            from_type,
            test_value,
            (to_types.as_ref(), false),
            &mut dm1,
            &mut dm2,
            &colnames,
        );

        api.schema_push_w_datasource(&dm1).send().assert_green();

        api.query(insert.into());

        tracing::info!("cast migration");
        api.schema_push_w_datasource(&dm2).send().assert_green();

        api.assert_schema().assert_table("Test", |table| {
            to_types.iter().enumerate().fold(
                table.assert_columns_count(to_types.len() + 1),
                |table, (idx, to_type)| {
                    table.assert_column(&colnames[idx], |col| col.assert_native_type(to_type, connector))
                },
            )
        });

        api.raw_cmd("DROP TABLE `Test`");
    }
}

#[test_connector(tags(Mysql))]
fn risky_casts_with_existing_data_should_warn(api: TestApi) {
    let connector = psl::builtin_connectors::MYSQL;
    let mut dm1 = String::with_capacity(256);
    let mut dm2 = String::with_capacity(256);
    let colnames = colnames_for_cases(RISKY_CASTS);
    let mut warnings: Vec<Cow<'_, str>> = Vec::with_capacity(6);
    let risky_casts = filter_from_types(&api, RISKY_CASTS);

    for (from_type, test_value, to_types) in risky_casts.iter() {
        let span = tracing::info_span!("RiskyCasts", from = %from_type, to = ?to_types);
        let _span = span.enter();

        let to_types = filter_to_types(&api, to_types);

        tracing::info!("initial migration");

        let insert = expand_cases(
            from_type,
            test_value,
            (to_types.as_ref(), false),
            &mut dm1,
            &mut dm2,
            &colnames,
        );

        warnings.clear();

        for (idx, to_type) in to_types.iter().enumerate() {
            let table = api.normalize_identifier("Test");

            warnings.push(format!(
                "You are about to alter the column `{column_name}` on the `{table}` table, which contains 1 non-null values. The data in that column will be cast from `{from}` to `{to}`.",
                column_name = colnames[idx],
                table = table,
                from = from_type,
                to = to_type,
            ).into());
        }

        api.schema_push_w_datasource(&dm1).send().assert_green();

        api.query(insert.into());

        tracing::info!("cast migration");

        api.schema_push_w_datasource(&dm2)
            .force(true)
            .send()
            .assert_executable()
            .assert_warnings(&warnings);

        api.assert_schema().assert_table("Test", |table| {
            to_types.iter().enumerate().fold(table, |table, (idx, to_type)| {
                table.assert_column(&colnames[idx], |col| col.assert_native_type(to_type, connector))
            })
        });

        api.raw_cmd("DROP TABLE `Test`");
    }
}

#[test_connector(tags(Mysql))]
fn impossible_casts_with_existing_data_should_warn(api: TestApi) {
    let connector = psl::builtin_connectors::MYSQL;
    let mut dm1 = String::with_capacity(256);
    let mut dm2 = String::with_capacity(256);
    let colnames = colnames_for_cases(IMPOSSIBLE_CASTS);
    let mut warnings: Vec<Cow<'_, str>> = Vec::with_capacity(6);
    let impossible_casts = filter_from_types(&api, IMPOSSIBLE_CASTS);

    for (from_type, test_value, to_types) in impossible_casts.iter() {
        let span = tracing::info_span!("ImpossibleCasts", from = %from_type, to = ?to_types);
        let _span = span.enter();

        let to_types = filter_to_types(&api, to_types);

        tracing::info!("initial migration");

        let insert = expand_cases(
            from_type,
            test_value,
            (to_types.as_ref(), true),
            &mut dm1,
            &mut dm2,
            &colnames,
        );

        warnings.clear();

        for (idx, _to_type) in to_types.iter().enumerate() {
            let table = api.normalize_identifier("Test");

            warnings.push(format!(
                "The `{column_name}` column on the `{table}` table would be dropped and recreated. This will lead to data loss.",
                table = table,
                column_name = colnames[idx],
                // from = from_type,
                // to = to_type,
            ).into());
        }

        api.schema_push_w_datasource(&dm1).send().assert_green();

        api.query(insert.into());

        tracing::info!("cast migration");

        api.schema_push_w_datasource(&dm2)
            .force(true)
            .send()
            .assert_executable()
            .assert_warnings(&warnings);

        api.assert_schema().assert_table("Test", |table| {
            to_types.iter().enumerate().fold(table, |table, (idx, to_type)| {
                table.assert_column(&colnames[idx], |col| col.assert_native_type(to_type, connector))
            })
        });

        api.raw_cmd("DROP TABLE `Test`");
    }
}

#[test_connector(tags(Mysql))]
fn typescript_starter_schema_with_native_types_is_idempotent(api: TestApi) {
    let dm = r#"
        model Post {
            id        Int     @id @default(autoincrement())
            title     String
            content   String?
            published Boolean @default(false)
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?
        }

        model User {
            id    Int     @id @default(autoincrement())
            email String  @unique
            name  String?
            posts Post[]
        }
    "#;

    let dm2 = r#"
        model Post {
            id        Int     @id @default(autoincrement()) @db.Int
            title     String  @db.VarChar(191)
            content   String? @db.VarChar(191)
            published Boolean @default(false) @db.TinyInt
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?    @db.Int
        }

        model User {
            id    Int     @id @default(autoincrement()) @db.Int
            email String  @unique @db.VarChar(191)
            name  String? @db.VarChar(191)
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

#[test_connector(tags(Mysql))]
fn typescript_starter_schema_with_different_native_types_is_idempotent(api: TestApi) {
    let dm = r#"
        model Post {
            id        Int     @id @default(autoincrement())
            title     String
            content   String?
            published Boolean @default(false)
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?
        }

        model User {
            id    Int     @id @default(autoincrement())
            email String  @unique
            name  String?
            posts Post[]
        }
    "#;

    let dm2 = r#"
        model Post {
            id        Int     @id @default(autoincrement()) @db.Int
            title     String  @db.VarChar(100)
            content   String? @db.VarChar(100)
            published Boolean @default(false) @db.TinyInt
            author    User?   @relation(fields: [authorId], references: [id])
            authorId  Int?    @db.Int
        }

        model User {
            id    Int     @id @default(autoincrement()) @db.Int
            email String  @unique @db.VarChar(100)
            name  String? @db.VarChar(100)
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
        .migration_id(Some("third")) // TODO (matthias) why does this work??
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(Mysql))]
fn time_zero_is_idempotent(api: TestApi) {
    let dm1 = indoc::indoc! {r#"
        model Class {
          id    Int      @id
          when  DateTime @db.Time(0)
        }
    "#};

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.schema_push_w_datasource(dm1)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(Mysql))]
fn time_is_idempotent(api: TestApi) {
    let dm1 = indoc::indoc! {r#"
        model Class {
          id    Int      @id
          when  DateTime @db.Time
        }
    "#};

    api.schema_push_w_datasource(dm1).send().assert_green();
    api.schema_push_w_datasource(dm1)
        .send()
        .assert_green()
        .assert_no_steps();
}
