#![allow(clippy::approx_constant)]

mod bigdecimal;

use crate::macros::assert_matching_value_and_column_type;
use crate::{connector::ColumnType, tests::test_api::*};
use std::str::FromStr;

test_type!(nvarchar_limited(
    mssql,
    "NVARCHAR(10)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(nvarchar_max(
    mssql,
    "NVARCHAR(max)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar"),
    Value::text("余"),
    Value::text("test¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥�😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁�🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀"),
));

test_type!(ntext(
    mssql,
    "NTEXT",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(varchar_limited(
    mssql,
    "VARCHAR(10)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar"),
));

test_type!(varchar_max(
    mssql,
    "VARCHAR(max)",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar"),
));

test_type!(text(
    mssql,
    "TEXT",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar")
));

test_type!(tinyint(
    mssql,
    "tinyint",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(u8::MIN),
    Value::int32(u8::MAX),
));

test_type!(smallint(
    mssql,
    "smallint",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(i16::MIN),
    Value::int32(i16::MAX),
));

test_type!(int(
    mssql,
    "int",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(i32::MIN),
    Value::int32(i32::MAX),
));

test_type!(bigint(
    mssql,
    "bigint",
    ColumnType::Int64,
    Value::null_int64(),
    Value::int64(i64::MIN),
    Value::int64(i64::MAX),
));

test_type!(float_24(
    mssql,
    "float(24)",
    ColumnType::Float,
    Value::null_float(),
    Value::float(1.23456),
));

test_type!(real(
    mssql,
    "real",
    ColumnType::Float,
    Value::null_float(),
    Value::float(1.123456)
));

test_type!(float_53(
    mssql,
    "float(53)",
    ColumnType::Double,
    Value::null_double(),
    Value::double(1.1234567891)
));

test_type!(money(
    mssql,
    "money",
    ColumnType::Double,
    Value::null_double(),
    Value::double(3.14)
));

test_type!(smallmoney(
    mssql,
    "smallmoney",
    ColumnType::Double,
    Value::null_double(),
    Value::double(3.14)
));

test_type!(boolean(
    mssql,
    "bit",
    ColumnType::Boolean,
    Value::null_boolean(),
    Value::boolean(true),
    Value::boolean(false),
));

test_type!(binary(
    mssql,
    "binary(8)",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(varbinary(
    mssql,
    "varbinary(8)",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(image(
    mssql,
    "image",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(date(
    mssql,
    "date",
    ColumnType::Date,
    Value::null_date(),
    Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap())
));

test_type!(time(
    mssql,
    "time",
    ColumnType::Time,
    Value::null_time(),
    Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap())
));

test_type!(datetime2(
    mssql,
    "datetime2",
    ColumnType::DateTime,
    Value::null_datetime(),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z").unwrap();
        Value::datetime(dt.with_timezone(&chrono::Utc))
    }
));

test_type!(datetime(
    mssql,
    "datetime",
    ColumnType::DateTime,
    Value::null_datetime(),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
        Value::datetime(dt.with_timezone(&chrono::Utc))
    }
));

test_type!(datetimeoffset(
    mssql,
    "datetimeoffset",
    ColumnType::DateTime,
    Value::null_datetime(),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
        Value::datetime(dt.with_timezone(&chrono::Utc))
    }
));

test_type!(smalldatetime(
    mssql,
    "smalldatetime",
    ColumnType::DateTime,
    Value::null_datetime(),
    {
        let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z").unwrap();
        Value::datetime(dt.with_timezone(&chrono::Utc))
    }
));

test_type!(uuid(
    mssql,
    "uniqueidentifier",
    ColumnType::Uuid,
    Value::null_uuid(),
    Value::uuid(uuid::Uuid::from_str("936DA01F-9ABD-4D9D-80C7-02AF85C822A8").unwrap())
));

test_type!(xml(
    mssql,
    "xml",
    ColumnType::Xml,
    Value::null_xml(),
    Value::xml("<foo>bar</foo>"),
));
