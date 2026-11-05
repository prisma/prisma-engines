use psl::builtin_connectors::KINGBASE_MYSQL;
use quaint::prelude::Queryable;
use sql_migration_tests::test_api::*;
use sql_schema_describer::ColumnTypeFamily;
use std::{borrow::Cow, fmt::Write};

/// (source native type, test value to insert, target native type)
type Case = (&'static str, quaint::ValueType<'static>, &'static [&'static str]);
type Cases = &'static [Case];

const SAFE_CASTS: Cases = &[
    (
        "BigInt",
        quaint::ValueType::Int64(Some(99999999432)),
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
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b"08088044"))),
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
        quaint::ValueType::Int32(Some(i32::MIN)),
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
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b""))),
        &[
            "SmallInt",
            "TinyInt",
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
        quaint::ValueType::Bytes(Some(Cow::Borrowed(&[0xff]))),
        &["TinyBlob", "MediumBlob", "LongBlob"],
    ),
    (
        "Char(10)",
        quaint::ValueType::Text(Some(Cow::Borrowed("1234"))),
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
        quaint::ValueType::Text(Some(Cow::Borrowed("2020-01-12"))),
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
        quaint::ValueType::Text(Some(Cow::Borrowed("2020-01-08 08:00:00"))),
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
        quaint::ValueType::Float(Some(3.20)),
        &[
            "Float",
            "Bit(64)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "Blob",
            // integers
            "Decimal(10,5)",
            "TinyInt",
            "Int",
            "Json",
            "UnsignedInt",
            "SmallInt",
            "MediumInt",
            "Year",
        ],
    ),
    (
        "Float",
        quaint::ValueType::Float(Some(3.20)),
        &[
            "Double",
            "Bit(32)",
            "LongText",
            "MediumText",
            "Text",
            "TinyText",
            "Blob",
            // integers
            "Decimal(10,5)",
            "TinyInt",
            "Int",
            "Json",
            "UnsignedInt",
            "SmallInt",
            "MediumInt",
            "Year",
            // Time
            "Time(0)",
        ],
    ),
    (
        "Json",
        quaint::ValueType::Text(Some(Cow::Borrowed("{\"a\":\"b\"}"))),
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
        quaint::ValueType::Bytes(Some(Cow::Borrowed(&[0xff]))),
        &["TinyBlob", "Blob", "MediumBlob"],
    ),
    (
        "MediumBlob",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(&[0xff]))),
        &["TinyBlob", "Blob", "LongBlob"],
    ),
    (
        "TinyBlob",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(&[0xff]))),
        &["LongBlob", "Blob", "MediumBlob"],
    ),
    (
        "Time",
        quaint::ValueType::Int32(Some(20)),
        &["VarChar(20)", "BigInt", "Int", "TinyInt", "Decimal(20,5)"],
    ),
    (
        "Year",
        quaint::ValueType::Int32(Some(2000)),
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
            "Float",
            "Double",
        ],
    ),
];

const RISKY_CASTS: Cases = &[
    (
        "BigInt",
        quaint::ValueType::Int64(Some(100)),
        &[
            "Int",
            "MediumInt",
            "SmallInt",
            "TinyInt",
            "UnsignedBigInt",
            "UnsignedInt",
        ],
    ),
    ("BigInt", quaint::ValueType::Int64(Some(2000)), &["Year"]),
    (
        "Binary(8)",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b"08088044"))),
        &["Bit(32)", "Int", "UnsignedBigInt", "UnsignedInt"],
    ),
    (
        "Binary(1)",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b"0"))),
        &["Time(0)", "SmallInt", "TinyInt"],
    ),
    (
        "Binary(4)",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b"2000"))),
        &["Year"],
    ),
    (
        "Bit(32)",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b""))),
        &["Decimal(10,2)", "Double", "Float"],
    ),
    (
        "Blob",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b"abc"))),
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
        quaint::ValueType::Text(Some(Cow::Borrowed("350"))),
        &["BigInt", "UnsignedBigInt", "Time(0)", "Json"],
    ),
    (
        "Double",
        quaint::ValueType::Float(Some(0f32)),
        &["Char(40)", "VarBinary(40)", "VarChar(40)"],
    ),
    (
        "Float",
        quaint::ValueType::Float(Some(0f32)),
        &["Char(40)", "VarBinary(40)", "VarChar(40)"],
    ),
    (
        "LongBlob",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b"abc"))),
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
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b"abc"))),
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
    ("SmallInt", quaint::ValueType::Int32(Some(1990)), &["Year", "Double"]),
    (
        "TinyBlob",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b"abc"))),
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
        quaint::ValueType::Int32(Some(5002)),
        &["Date", "DateTime(0)", "Timestamp(0)"],
    ),
    (
        "Year",
        quaint::ValueType::Text(Some(Cow::Borrowed("1999"))),
        &["Decimal(10,0)", "Json"],
    ),
];

const IMPOSSIBLE_CASTS: Cases = &[
    (
        "BigInt",
        quaint::ValueType::Int64(Some(500)),
        &["Decimal(15,6)", "Date", "DateTime(0)", "Json", "Timestamp(0)"],
    ),
    (
        "Binary(12)",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b"8080008"))),
        &["Date", "DateTime(0)", "Json", "Timestamp(0)"],
    ),
    (
        "Bit(32)",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(b""))),
        &["Date", "DateTime(0)", "Time(0)", "Timestamp(0)", "Json"],
    ),
    (
        "Blob",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(&[0x00]))),
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
            "UnsignedBigInt",
            "Year",
        ],
    ),
    (
        "Date",
        quaint::ValueType::Text(Some(Cow::Borrowed("2020-01-12"))),
        &["TinyInt", "Year", "SmallInt", "MediumInt"],
    ),
    (
        "DateTime(0)",
        quaint::ValueType::Text(Some(Cow::Borrowed("2020-01-08 08:00:00"))),
        &["TinyInt", "Int", "UnsignedInt", "SmallInt", "MediumInt", "Year"],
    ),
    (
        "Double",
        quaint::ValueType::Float(Some(3.20)),
        &["Binary(10)", "Date", "Timestamp(0)", "DateTime(0)"],
    ),
    (
        "Float",
        quaint::ValueType::Float(Some(3.20)),
        &["Binary(10)", "Date", "Timestamp(0)", "DateTime(0)"],
    ),
    (
        "Json",
        quaint::ValueType::Text(Some(Cow::Borrowed("{\"a\":\"b\"}"))),
        &[
            // Integer types
            "Bit(64)",
            "Int",
            "MediumInt",
            "SmallInt",
            "TinyInt",
            "UnsignedBigInt",
            "UnsignedInt",
            "Float",
            "Double",
        ],
    ),
    (
        "LongBlob",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(&[0x00]))),
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
            "UnsignedBigInt",
            "Year",
        ],
    ),
    (
        "MediumBlob",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(&[0x00]))),
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
            "UnsignedBigInt",
            "Year",
        ],
    ),
    ("Time(0)", quaint::ValueType::Int32(Some(0)), &["Json", "Year"]),
    (
        "TinyBlob",
        quaint::ValueType::Bytes(Some(Cow::Borrowed(&[0x00]))),
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
            "UnsignedBigInt",
            "Year",
        ],
    ),
    (
        "Year",
        quaint::ValueType::Int32(Some(2001)),
        &["TinyInt", "Date", "Time(0)", "DateTime(0)", "Timestamp(0)"],
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
        ("VarBinary", "Bytes"),
        ("VarChar", "String"),
        ("Year", "Int"),
    ];

    let scalar_type =
        scalar_type.trim_end_matches(|ch: char| [' ', ',', '(', ')'].contains(&ch) || ch.is_ascii_digit());

    let idx = TYPES_MAP
        .binary_search_by_key(&scalar_type, |(native, _prisma)| native)
        .map_err(|_err| format!("Could not find {scalar_type} in TYPES_MAP"))
        .unwrap();

    TYPES_MAP[idx].1
}

fn colnames_for_cases(cases: Cases) -> Vec<String> {
    let max_colname = cases.iter().map(|(_, _, to_types)| to_types.len()).max().unwrap();

    std::iter::repeat(())
        .enumerate()
        .take(max_colname)
        .map(|(idx, _)| format!("col{idx}"))
        .collect()
}

fn native_type_base(native_type: &str) -> &str {
    native_type.split('(').next().unwrap()
}

fn is_blob_type(native_type: &str) -> bool {
    matches!(
        native_type_base(native_type),
        "TinyBlob" | "Blob" | "MediumBlob" | "LongBlob"
    )
}

fn is_binary_type(native_type: &str) -> bool {
    matches!(native_type_base(native_type), "Bit" | "Binary" | "VarBinary") || is_blob_type(native_type)
}

fn is_bit_type(native_type: &str) -> bool {
    native_type_base(native_type) == "Bit"
}

fn is_string_type(native_type: &str) -> bool {
    matches!(
        native_type_base(native_type),
        "Char" | "VarChar" | "TinyText" | "Text" | "MediumText" | "LongText"
    )
}

fn is_numeric_type(native_type: &str) -> bool {
    matches!(
        native_type_base(native_type),
        "Int"
            | "UnsignedInt"
            | "SmallInt"
            | "TinyInt"
            | "MediumInt"
            | "BigInt"
            | "UnsignedBigInt"
            | "Decimal"
            | "Float"
            | "Double"
            | "Year"
    )
}

fn is_datetime_type(native_type: &str) -> bool {
    matches!(
        native_type_base(native_type),
        "Date" | "Time" | "DateTime" | "Timestamp"
    )
}

fn explicit_cast_supported_by_names(previous: &str, next: &str) -> bool {
    if is_blob_type(previous) && !is_blob_type(next) {
        return is_string_type(next) || is_binary_type(next);
    }

    if native_type_base(previous) == "Json" && (is_numeric_type(next) || is_datetime_type(next) || is_bit_type(next)) {
        return false;
    }

    if is_binary_type(previous)
        && (is_datetime_type(next) || native_type_base(next) == "Json" || is_numeric_type(next) || is_bit_type(next))
    {
        return false;
    }

    if is_numeric_type(previous) && is_datetime_type(next) {
        return false;
    }

    if is_numeric_type(previous) && native_type_base(next) == "Year" {
        return false;
    }

    if is_datetime_type(previous) && (native_type_base(next) == "Json" || is_bit_type(next)) {
        return false;
    }

    true
}

fn kingbase_cast_kind(from_type: &str, to_type: &str, cast_kind: CastKind) -> CastKind {
    match cast_kind {
        // Kingbase does not implicitly cast most of the conversions that are
        // safe in MySQL. Keep the matrix entry, but expect the explicit USING
        // path and its risky warning. Int -> VarChar is the one safe cast
        // special-cased by the Kingbase schema differ.
        CastKind::Safe if native_type_base(from_type) == "Int" && native_type_base(to_type) == "VarChar" => {
            CastKind::Safe
        }
        CastKind::Safe => CastKind::Risky,
        other => other,
    }
}

fn expected_native_type(native_type: &str) -> &str {
    warning_native_type(native_type)
}

fn expand_cases<'a, 'b>(
    from_type: &str,
    test_value: &'a quaint::ValueType<'a>,
    (to_types, nullable): (&[&str], bool),
    dm1: &'b mut String,
    dm2: &'b mut String,
    colnames: &'a [String],
) -> String {
    let mut values = Vec::with_capacity(to_types.len());

    for dm in std::iter::once(&mut *dm1).chain(std::iter::once(&mut *dm2)) {
        dm.clear();
        dm.push_str("model Test {\nid Int @id @default(autoincrement())\n");
    }

    for (idx, _) in to_types.iter().enumerate() {
        writeln!(
            dm1,
            "{colname} {scalar_type}{nullability} @db.{native_type}",
            colname = colnames[idx],
            scalar_type = native_type_name_to_prisma_scalar_type_name(from_type),
            native_type = from_type,
            nullability = if nullable { "?" } else { "" },
        )
        .unwrap();

        values.push(sql_literal(from_type, test_value));
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
    }

    for dm in std::iter::once(&mut *dm1).chain(std::iter::once(&mut *dm2)) {
        dm.push('}');
    }

    format!(
        "INSERT INTO `Test` ({}) VALUES ({})",
        colnames[..to_types.len()]
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", "),
        values.join(", ")
    )
}

fn sql_literal(from_type: &str, value: &quaint::ValueType<'_>) -> String {
    // These cases exercise schema casts, not the still-specialized Kingbase bind codecs.
    match value {
        quaint::ValueType::Int32(Some(value)) => value.to_string(),
        quaint::ValueType::Int64(Some(value)) => value.to_string(),
        quaint::ValueType::Float(Some(value)) => value.to_string(),
        quaint::ValueType::Double(Some(value)) => value.to_string(),
        quaint::ValueType::Text(Some(value)) => format!("'{}'", value.as_ref().replace('\'', "''")),
        quaint::ValueType::Bytes(Some(value)) => {
            let hex = value.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
            if matches!(from_type, "TinyBlob" | "Blob" | "MediumBlob" | "LongBlob") {
                format!("CAST(x'{hex}' AS BLOB)")
            } else {
                format!("x'{hex}'")
            }
        }
        other => panic!("Unsupported Kingbase MySQL test value: {other:?}"),
    }
}

fn warning_native_type(native_type: &str) -> &str {
    match native_type {
        "Time" => "Time(0)",
        native_type => native_type,
    }
}

#[derive(Clone, Copy)]
enum CastKind {
    Safe,
    Risky,
    Impossible,
}

struct CastBuffers<'a> {
    dm1: String,
    dm2: String,
    warnings: Vec<Cow<'a, str>>,
}

fn run_casts_with_existing_data(api: &mut TestApi, cases: Cases, cast_kind: CastKind) {
    let colnames = colnames_for_cases(cases);
    let mut buffers = CastBuffers {
        dm1: String::with_capacity(256),
        dm2: String::with_capacity(256),
        warnings: Vec::new(),
    };

    for (from_type, test_value, to_types) in cases {
        let (native_temporal, to_types): (Vec<_>, Vec<_>) = to_types
            .iter()
            .copied()
            .partition(|to_type| is_datetime_type(from_type) && is_numeric_type(to_type));
        for target in native_temporal {
            run_native_temporal_cast(api, from_type, test_value, target, &colnames, &mut buffers);
        }
        let (no_op, remaining): (Vec<_>, Vec<_>) = to_types.into_iter().partition(|to_type| *from_type == *to_type);
        let mut groups = Vec::new();

        // Only identical native types are no-ops.
        if !no_op.is_empty() {
            groups.push((no_op, CastKind::Safe));
        }

        let (explicit, destructive): (Vec<_>, Vec<_>) = remaining
            .into_iter()
            .partition(|to_type| explicit_cast_supported_by_names(from_type, to_type));

        match cast_kind {
            CastKind::Safe => {
                let (safe, risky): (Vec<_>, Vec<_>) = explicit
                    .into_iter()
                    .partition(|to_type| matches!(kingbase_cast_kind(from_type, to_type, cast_kind), CastKind::Safe));

                if !safe.is_empty() {
                    groups.push((safe, CastKind::Safe));
                }
                if !risky.is_empty() {
                    groups.push((risky, CastKind::Risky));
                }
            }
            CastKind::Risky | CastKind::Impossible if !explicit.is_empty() => {
                // MySQL's impossible casts are kept as coverage, but Kingbase
                // can preserve data whenever a USING expression exists.
                groups.push((explicit, CastKind::Risky));
            }
            _ => (),
        }

        if !destructive.is_empty() {
            groups.push((destructive, CastKind::Impossible));
        }

        for (to_types, effective_kind) in groups {
            run_cast_group(
                api,
                from_type,
                test_value,
                &to_types,
                effective_kind,
                &colnames,
                &mut buffers,
            );
        }
    }
}

// The server is the independent oracle for temporal conversions. In
// particular insufficient target capacity must fail, never delete the column.
fn run_native_temporal_cast(
    api: &mut TestApi,
    source: &str,
    value: &quaint::ValueType<'_>,
    target: &str,
    columns: &[String],
    buffers: &mut CastBuffers<'_>,
) {
    let sql_type = match target {
        "UnsignedInt" => "sys.uint4".to_owned(),
        "UnsignedBigInt" => "sys.uint8".to_owned(),
        _ => target.to_ascii_uppercase(),
    };
    let insert = expand_cases(
        source,
        value,
        (&[target], false),
        &mut buffers.dm1,
        &mut buffers.dm2,
        columns,
    );
    api.schema_push_w_datasource(buffers.dm1.clone()).send().assert_green();
    api.raw_cmd(&insert);
    let conn = tok(quaint::single::Quaint::new(api.connection_string())).unwrap();
    let original = tok(conn.query_raw("SELECT col0::text AS value FROM `Test`", &[])).unwrap();
    let expected = tok(conn.query_raw(
        &format!("SELECT CAST(CAST(col0 AS {sql_type}) AS TEXT) AS value FROM `Test`"),
        &[],
    ));
    match expected {
        Ok(expected) => {
            api.schema_push_w_datasource(buffers.dm2.clone())
                .force(true)
                .send()
                .assert_executable()
                .assert_has_executed_steps();
            let actual = tok(conn.query_raw("SELECT col0::text AS value FROM `Test`", &[])).unwrap();
            assert_eq!(
                actual.first().unwrap()[0],
                expected.first().unwrap()[0],
                "{source} -> {target}"
            );
            api.schema_push_w_datasource(buffers.dm2.clone())
                .send()
                .assert_green()
                .assert_no_steps();
        }
        Err(native_error) => {
            let error = api
                .schema_push_w_datasource(buffers.dm2.clone())
                .force(true)
                .send_unwrap_err();
            assert!(
                error.to_string().contains(native_error.original_message().unwrap()),
                "{source} -> {target}: {error}, native: {native_error}"
            );
            let actual = tok(conn.query_raw("SELECT col0::text AS value FROM `Test`", &[])).unwrap();
            assert_eq!(actual.first().unwrap()[0], original.first().unwrap()[0]);
            api.schema_push_w_datasource(buffers.dm1.clone())
                .send()
                .assert_green()
                .assert_no_steps();
        }
    }
    api.raw_cmd("DROP TABLE `Test`");
}

fn run_cast_group(
    api: &mut TestApi,
    from_type: &str,
    test_value: &quaint::ValueType<'_>,
    to_types: &[&str],
    cast_kind: CastKind,
    colnames: &[String],
    buffers: &mut CastBuffers<'_>,
) {
    let CastBuffers { dm1, dm2, warnings } = buffers;
    let insert = expand_cases(
        from_type,
        test_value,
        (to_types, matches!(cast_kind, CastKind::Impossible)),
        dm1,
        dm2,
        colnames,
    );

    warnings.clear();
    for (idx, to_type) in to_types.iter().enumerate() {
        let table = api.normalize_identifier("Test");
        let warning: Cow<'_, str> = match cast_kind {
            CastKind::Safe => continue,
            CastKind::Risky => format!(
                "You are about to alter the column `{}` on the `{}` table, which contains 1 non-null values. The data in that column will be cast from `{from_type}` to `{to_type}`.",
                colnames[idx], table, from_type = warning_native_type(from_type)
            )
            .into(),
            CastKind::Impossible => format!(
                "The `{}` column on the `{}` table would be dropped and recreated. This will lead to data loss.",
                colnames[idx], table
            )
            .into(),
        };
        warnings.push(warning);
    }

    api.schema_push_w_datasource(dm1.clone()).send().assert_green();
    api.raw_cmd(&insert);

    match cast_kind {
        CastKind::Safe => api.schema_push_w_datasource(dm2.clone()).send().assert_green(),
        CastKind::Risky => api
            .schema_push_w_datasource(dm2.clone())
            .force(true)
            .send()
            .assert_executable()
            .assert_warnings(warnings)
            .assert_has_executed_steps(),
        CastKind::Impossible => api
            .schema_push_w_datasource(dm2.clone())
            .force(true)
            .send()
            .assert_executable()
            .assert_warnings(warnings)
            .assert_has_executed_steps(),
    };

    api.assert_schema().assert_table("Test", |table| {
        to_types.iter().enumerate().fold(
            table.assert_columns_count(to_types.len() + 1),
            |table, (idx, to_type)| {
                table.assert_column(&colnames[idx], |column| {
                    column.assert_native_type(expected_native_type(to_type), KINGBASE_MYSQL)
                })
            },
        )
    });

    api.raw_cmd("DROP TABLE `Test`");
}

#[test_connector(tags(KingbaseMysql))]
fn scalar_defaults_and_native_types_can_be_migrated(api: TestApi) {
    let schema = r#"
        model Defaults {
            id        Int      @id
            amount    Decimal  @default(12.34) @db.Decimal(8, 2)
            enabled   Boolean  @default(true)
            name      String   @default("O'Reilly\\path") @db.VarChar(64)
            createdAt DateTime @default(now()) @db.DateTime(3)
            payload   Bytes?   @db.VarBinary(16)
            metadata  Json?
        }
    "#;

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Defaults", |table| {
        table
            .assert_column("amount", |column| column.assert_type_family(ColumnTypeFamily::Decimal))
            .assert_column("enabled", |column| column.assert_type_family(ColumnTypeFamily::Boolean))
            .assert_column("name", |column| column.assert_type_family(ColumnTypeFamily::String))
            .assert_column("createdAt", |column| {
                column.assert_type_family(ColumnTypeFamily::DateTime)
            })
            .assert_column("payload", |column| column.assert_type_family(ColumnTypeFamily::Binary))
            .assert_column("metadata", |column| column.assert_type_family(ColumnTypeFamily::Json))
    });

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseMysql))]
fn scalar_column_types_and_defaults_can_be_changed(api: TestApi) {
    let initial_schema = r#"
        model Settings {
            id    Int     @id
            value Decimal @default(1.50) @db.Decimal(5, 2)
            label String  @default("draft") @db.VarChar(16)
        }
    "#;

    api.schema_push_w_datasource(initial_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    let changed_schema = r#"
        model Settings {
            id    Int     @id
            value Decimal @default(2.50) @db.Decimal(8, 2)
            label String  @default("published") @db.VarChar(32)
        }
    "#;

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.assert_schema().assert_table("Settings", |table| {
        table
            .assert_column("value", |column| column.assert_type_family(ColumnTypeFamily::Decimal))
            .assert_column("label", |column| column.assert_type_family(ColumnTypeFamily::String))
    });

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseMysql))]
fn all_supported_native_types_can_be_created_and_reintrospected(api: TestApi) {
    let schema = r#"
        model NativeTypes {
            id                Int      @id @db.Int
            intValue          Int?     @db.Int
            unsignedInt       Int?     @db.UnsignedInt
            smallInt          Int?     @db.SmallInt
            tinyInt           Int?     @db.TinyInt
            mediumInt         Int?     @db.MediumInt
            bigInt             BigInt?  @db.BigInt
            unsignedBigInt     BigInt?  @db.UnsignedBigInt
            decimalValue       Decimal? @db.Decimal(5, 3)
            floatValue         Float?   @db.Float
            doubleValue        Float?   @db.Double
            tinyIntBool        Boolean? @db.TinyInt
            bitBool            Boolean? @db.Bit(1)
            bitValue           Bytes?   @db.Bit(8)
            charValue          String?  @db.Char(10)
            varcharValue       String?  @db.VarChar(32)
            binaryValue        Bytes?   @db.Binary(8)
            varbinaryValue     Bytes?   @db.VarBinary(8)
            tinyBlob           Bytes?   @db.TinyBlob
            blobValue          Bytes?   @db.Blob
            mediumBlob         Bytes?   @db.MediumBlob
            longBlob           Bytes?   @db.LongBlob
            tinyText           String?  @db.TinyText
            textValue          String?  @db.Text
            mediumText         String?  @db.MediumText
            longText           String?  @db.LongText
            dateValue          DateTime? @db.Date
            timeValue          DateTime? @db.Time(3)
            datetimeValue      DateTime? @db.DateTime(3)
            timestampValue     DateTime? @db.Timestamp(3)
            yearValue          Int?      @db.Year
            jsonValue          Json?     @db.Json
        }
    "#;

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.raw_cmd(
        "INSERT INTO `NativeTypes` (`id`, `unsignedInt`, `unsignedBigInt`) VALUES (1, 4294967295, 9223372036854775807)",
    );
    api.assert_schema().assert_table("NativeTypes", |table| {
        let expected_native_types = [
            ("id", Some("Int")),
            ("intValue", Some("Int")),
            // Only server-native uint4/uint8 are supported unsigned types.
            ("unsignedInt", Some("UnsignedInt")),
            ("smallInt", Some("SmallInt")),
            ("tinyInt", Some("TinyInt")),
            ("mediumInt", Some("MediumInt")),
            ("bigInt", Some("BigInt")),
            ("unsignedBigInt", Some("UnsignedBigInt")),
            ("decimalValue", Some("Decimal(5,3)")),
            ("floatValue", Some("Float")),
            ("doubleValue", Some("Double")),
            ("tinyIntBool", None),
            ("bitBool", Some("Bit(1)")),
            ("bitValue", Some("Bit(8)")),
            ("charValue", Some("Char(10)")),
            ("varcharValue", Some("VarChar(32)")),
            ("binaryValue", Some("Binary(8)")),
            ("varbinaryValue", Some("VarBinary(8)")),
            ("tinyBlob", Some("TinyBlob")),
            ("blobValue", Some("Blob")),
            ("mediumBlob", Some("MediumBlob")),
            ("longBlob", Some("LongBlob")),
            ("tinyText", Some("TinyText")),
            ("textValue", Some("Text")),
            ("mediumText", Some("MediumText")),
            ("longText", Some("LongText")),
            ("dateValue", Some("Date")),
            ("timeValue", Some("Time(3)")),
            ("datetimeValue", Some("DateTime(3)")),
            ("timestampValue", Some("Timestamp(3)")),
            ("yearValue", Some("Year")),
            ("jsonValue", Some("Json")),
        ];

        expected_native_types
            .into_iter()
            .fold(table.assert_columns_count(32), |table, (name, native_type)| {
                table.assert_column(name, |column| match native_type {
                    Some(native_type) => column.assert_native_type(native_type, KINGBASE_MYSQL),
                    None => column.assert_type_family(ColumnTypeFamily::Boolean),
                })
            })
    });

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseMysql))]
fn safe_casts_with_existing_data_should_work(mut api: TestApi) {
    run_casts_with_existing_data(&mut api, SAFE_CASTS, CastKind::Safe);
}

#[test_connector(tags(KingbaseMysql))]
fn risky_casts_with_existing_data_should_warn(mut api: TestApi) {
    run_casts_with_existing_data(&mut api, RISKY_CASTS, CastKind::Risky);
}

#[test_connector(tags(KingbaseMysql))]
fn not_castable_with_existing_data_should_warn(mut api: TestApi) {
    run_casts_with_existing_data(&mut api, IMPOSSIBLE_CASTS, CastKind::Impossible);
}

#[test_connector(tags(KingbaseMysql))]
fn starter_schema_without_native_type_annotations_is_idempotent(api: TestApi) {
    let schema = r#"
        model Post {
            id        Int     @id
            title     String
            content   String?
            published Boolean @default(false)
        }
    "#;

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseMysql))]
fn starter_schema_with_native_type_annotations_is_idempotent(api: TestApi) {
    let initial_schema = r#"
        model Post {
            id        Int     @id
            title     String
            content   String?
            published Boolean @default(false)
        }
    "#;

    let annotated_schema = r#"
        model Post {
            id        Int     @id @db.Int
            title     String  @db.VarChar(191)
            content   String? @db.VarChar(191)
            published Boolean @default(false) @db.TinyInt
        }
    "#;

    api.schema_push_w_datasource(initial_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(annotated_schema)
        .send()
        .assert_green()
        .assert_no_steps();
    api.schema_push_w_datasource(annotated_schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseMysql))]
fn time_zero_is_idempotent(api: TestApi) {
    let schema = r#"
        model Event {
            id  Int      @id
            at  DateTime @db.Time(0)
        }
    "#;

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}

#[test_connector(tags(KingbaseMysql))]
fn time_is_idempotent(api: TestApi) {
    let schema = r#"
        model Event {
            id  Int      @id
            at  DateTime @db.Time
        }
    "#;

    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();
    api.schema_push_w_datasource(schema)
        .send()
        .assert_green()
        .assert_no_steps();
}
