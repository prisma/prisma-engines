#![allow(clippy::approx_constant)]

#[cfg(feature = "bigdecimal")]
mod bigdecimal;

use crate::tests::test_api::*;

test_type!(nvarchar_limited(
    mssql,
    "NVARCHAR(10)",
    ValueType::Text(None).into_value(),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(nvarchar_max(
    mssql,
    "NVARCHAR(max)",
    ValueType::Text(None).into_value(),
    Value::text("foobar"),
    Value::text("余"),
    Value::text("test¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥�😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁�🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀"),
));

test_type!(ntext(
    mssql,
    "NTEXT",
    ValueType::Text(None).into_value(),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(varchar_limited(
    mssql,
    "VARCHAR(10)",
    ValueType::Text(None).into_value(),
    Value::text("foobar"),
));

test_type!(varchar_max(
    mssql,
    "VARCHAR(max)",
    ValueType::Text(None).into_value(),
    Value::text("foobar"),
));

test_type!(text(mssql, "TEXT", ValueType::Text(None).into_value(), Value::text("foobar")));

test_type!(tinyint(
    mssql,
    "tinyint",
    ValueType::Int32(None).into_value(),
    Value::int32(u8::MIN),
    Value::int32(u8::MAX),
));

test_type!(smallint(
    mssql,
    "smallint",
    ValueType::Int32(None).into_value(),
    Value::int32(i16::MIN),
    Value::int32(i16::MAX),
));

test_type!(int(
    mssql,
    "int",
    ValueType::Int32(None).into_value(),
    Value::int32(i32::MIN),
    Value::int32(i32::MAX),
));

test_type!(bigint(
    mssql,
    "bigint",
    ValueType::Int64(None).into_value(),
    Value::int64(i64::MIN),
    Value::int64(i64::MAX),
));

test_type!(float_24(
    mssql,
    "float(24)",
    ValueType::Float(None).into_value(),
    Value::float(1.23456),
));

test_type!(real(mssql, "real", ValueType::Float(None).into_value(), Value::float(1.123456)));

test_type!(float_53(
    mssql,
    "float(53)",
    ValueType::Double(None).into_value(),
    Value::double(1.1234567891)
));

test_type!(money(mssql, "money", ValueType::Double(None).into_value(), Value::double(3.14)));

test_type!(smallmoney(
    mssql,
    "smallmoney",
    ValueType::Double(None).into_value(),
    Value::double(3.14)
));

test_type!(boolean(
    mssql,
    "bit",
    ValueType::Boolean(None).into_value(),
    Value::boolean(true),
    Value::boolean(false),
));

test_type!(binary(
    mssql,
    "binary(8)",
    ValueType::Bytes(None).into_value(),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(varbinary(
    mssql,
    "varbinary(8)",
    ValueType::Bytes(None).into_value(),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(image(
    mssql,
    "image",
    ValueType::Bytes(None).into_value(),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(date(
    mssql,
    "date",
    ValueType::Date(None).into_value(),
    Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap())
));

test_type!(time(
    mssql,
    "time",
    ValueType::Time(None).into_value(),
    Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap())
));

test_type!(datetime2(mssql, "datetime2", ValueType::DateTime(None).into_value(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(datetime(mssql, "datetime", ValueType::DateTime(None).into_value(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(datetimeoffset(mssql, "datetimeoffset", ValueType::DateTime(None).into_value(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(smalldatetime(mssql, "smalldatetime", ValueType::DateTime(None).into_value(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));
