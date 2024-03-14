#![allow(clippy::approx_constant)]

mod bigdecimal;

use crate::tests::test_api::*;

test_type!(nvarchar_limited(
    mssql,
    "NVARCHAR(10)",
    Value::null_text(),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(nvarchar_max(
    mssql,
    "NVARCHAR(max)",
    Value::null_text(),
    Value::text("foobar"),
    Value::text("余"),
    Value::text("test¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥�😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁�🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀"),
));

test_type!(ntext(
    mssql,
    "NTEXT",
    Value::null_text(),
    Value::text("foobar"),
    Value::text("余"),
));

test_type!(varchar_limited(
    mssql,
    "VARCHAR(10)",
    Value::null_text(),
    Value::text("foobar"),
));

test_type!(varchar_max(
    mssql,
    "VARCHAR(max)",
    Value::null_text(),
    Value::text("foobar"),
));

test_type!(text(mssql, "TEXT", Value::null_text(), Value::text("foobar")));

test_type!(tinyint(
    mssql,
    "tinyint",
    Value::null_int32(),
    Value::int32(u8::MIN),
    Value::int32(u8::MAX),
));

test_type!(smallint(
    mssql,
    "smallint",
    Value::null_int32(),
    Value::int32(i16::MIN),
    Value::int32(i16::MAX),
));

test_type!(int(
    mssql,
    "int",
    Value::null_int32(),
    Value::int32(i32::MIN),
    Value::int32(i32::MAX),
));

test_type!(bigint(
    mssql,
    "bigint",
    Value::null_int64(),
    Value::int64(i64::MIN),
    Value::int64(i64::MAX),
));

test_type!(float_24(mssql, "float(24)", Value::null_float(), Value::float(1.23456),));

test_type!(real(mssql, "real", Value::null_float(), Value::float(1.123456)));

test_type!(float_53(
    mssql,
    "float(53)",
    Value::null_double(),
    Value::double(1.1234567891)
));

test_type!(money(mssql, "money", Value::null_double(), Value::double(3.14)));

test_type!(smallmoney(
    mssql,
    "smallmoney",
    Value::null_double(),
    Value::double(3.14)
));

test_type!(boolean(
    mssql,
    "bit",
    Value::null_boolean(),
    Value::boolean(true),
    Value::boolean(false),
));

test_type!(binary(
    mssql,
    "binary(8)",
    Value::null_bytes(),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(varbinary(
    mssql,
    "varbinary(8)",
    Value::null_bytes(),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(image(
    mssql,
    "image",
    Value::null_bytes(),
    Value::bytes(b"DEADBEEF".to_vec()),
));

test_type!(date(
    mssql,
    "date",
    Value::null_date(),
    Value::date(chrono::NaiveDate::from_ymd_opt(2020, 4, 20).unwrap())
));

test_type!(time(
    mssql,
    "time",
    Value::null_time(),
    Value::time(chrono::NaiveTime::from_hms_opt(16, 20, 00).unwrap())
));

test_type!(datetime2(mssql, "datetime2", Value::null_datetime(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(datetime(mssql, "datetime", Value::null_datetime(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(datetimeoffset(mssql, "datetimeoffset", Value::null_datetime(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:22Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));

test_type!(smalldatetime(mssql, "smalldatetime", Value::null_datetime(), {
    let dt = chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z").unwrap();
    Value::datetime(dt.with_timezone(&chrono::Utc))
}));
