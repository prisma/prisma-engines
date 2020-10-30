#[cfg(feature = "bigdecimal")]
mod bigdecimal;

use crate::tests::test_api::*;

test_type!(nvarchar_limited(
    mssql,
    "NVARCHAR(10)",
    Value::Text(None),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(nvarchar_max(
    mssql,
    "NVARCHAR(max)",
    Value::Text(None),
    Value::text("foobar"),
    Value::text("余"),
    Value::text("test¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥�😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁�🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀"),
));

test_type!(ntext(
    mssql,
    "NTEXT",
    Value::Text(None),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(varchar_limited(
    mssql,
    "VARCHAR(10)",
    Value::Text(None),
    Value::text("foobar"),
));

test_type!(varchar_max(
    mssql,
    "VARCHAR(max)",
    Value::Text(None),
    Value::text("foobar"),
));

test_type!(text(mssql, "TEXT", Value::Text(None), Value::text("foobar")));

test_type!(tinyint(
    mssql,
    "tinyint",
    Value::Integer(None),
    Value::integer(u8::MIN),
    Value::integer(u8::MAX),
));

test_type!(smallint(
    mssql,
    "smallint",
    Value::Integer(None),
    Value::integer(i16::MIN),
    Value::integer(i16::MAX),
));

test_type!(int(
    mssql,
    "int",
    Value::Integer(None),
    Value::integer(i32::MIN),
    Value::integer(i32::MAX),
));

test_type!(bigint(
    mssql,
    "bigint",
    Value::Integer(None),
    Value::integer(i64::MIN),
    Value::integer(i64::MAX),
));

test_type!(float_24(mssql, "float(24)", Value::Float(None), Value::float(1.23456),));

test_type!(real(mssql, "real", Value::Float(None), Value::float(1.123456)));

test_type!(float_53(
    mssql,
    "float(53)",
    Value::Float(None),
    Value::double(1.1234567891)
));

test_type!(money(mssql, "money", Value::Double(None), Value::double(3.14)));

test_type!(smallmoney(
    mssql,
    "smallmoney",
    Value::Double(None),
    Value::double(3.14)
));

test_type!(boolean(
    mssql,
    "bit",
    Value::Boolean(None),
    Value::boolean(true),
    Value::boolean(false),
));

test_type!(binary(
    mssql,
    "binary(8)",
    Value::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(varbinary(
    mssql,
    "varbinary(8)",
    Value::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(image(
    mssql,
    "image",
    Value::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec()),
));

#[cfg(feature = "chrono")]
test_type!(date(
    mssql,
    "date",
    Value::Date(None),
    Value::date(chrono::NaiveDate::from_ymd(2020, 4, 20))
));

#[cfg(feature = "chrono")]
test_type!(time(
    mssql,
    "time",
    Value::Time(None),
    Value::time(chrono::NaiveTime::from_hms(16, 20, 00))
));

#[cfg(feature = "chrono")]
test_type!(datetime2(mssql, "datetime2", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono")]
test_type!(datetime(mssql, "datetime", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono")]
test_type!(datetimeoffset(mssql, "datetimeoffset", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

#[cfg(feature = "chrono")]
test_type!(smalldatetime(mssql, "smalldatetime", Value::DateTime(None), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));
