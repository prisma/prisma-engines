#![allow(clippy::approx_constant)]

use crate::connector::ColumnType;
use crate::macros::assert_matching_value_and_column_type;
use crate::tests::test_api::TestApi;
use crate::tests::test_api::sqlite_test_api;
use crate::{ast::*, connector::Queryable};

use std::str::FromStr;

test_type!(integer(
    sqlite,
    "INTEGER",
    ColumnType::Int32,
    Value::null_int32(),
    Value::int32(i8::MIN),
    Value::int32(i8::MAX),
    Value::int32(i16::MIN),
    Value::int32(i16::MAX),
    Value::int32(i32::MIN),
    Value::int32(i32::MAX),
));

test_type!(big_int(
    sqlite,
    "BIGINT",
    ColumnType::Int64,
    Value::null_int64(),
    Value::int64(i64::MIN),
    Value::int64(i64::MAX),
));

test_type!(real(
    sqlite,
    "REAL",
    ColumnType::Double,
    Value::null_double(),
    Value::double(1.12345)
));

test_type!(float_decimal(
    sqlite,
    "FLOAT",
    ColumnType::Double,
    (Value::null_numeric(), Value::null_double()),
    (
        Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap()),
        Value::double(3.14)
    )
));

test_type!(double_decimal(
    sqlite,
    "DOUBLE",
    ColumnType::Double,
    (Value::null_numeric(), Value::null_double()),
    (
        Value::numeric(bigdecimal::BigDecimal::from_str("3.14").unwrap()),
        Value::double(3.14)
    )
));

test_type!(text(
    sqlite,
    "TEXT",
    ColumnType::Text,
    Value::null_text(),
    Value::text("foobar huhuu")
));

test_type!(blob(
    sqlite,
    "BLOB",
    ColumnType::Bytes,
    Value::null_bytes(),
    Value::bytes(b"DEADBEEF".to_vec())
));

test_type!(float(
    sqlite,
    "FLOAT",
    ColumnType::Double,
    (Value::null_float(), Value::null_double()),
    (Value::null_double(), Value::null_double()),
    (Value::float(1.23456), Value::double(1.23456)),
    (Value::double(1.2312313213), Value::double(1.2312313213))
));

test_type!(double(
    sqlite,
    "DOUBLE",
    ColumnType::Double,
    Value::null_double(),
    Value::double(1.2312313213),
));

test_type!(boolean(
    sqlite,
    "BOOLEAN",
    ColumnType::Boolean,
    Value::null_boolean(),
    Value::boolean(true),
    Value::boolean(false)
));

test_type!(date(
    sqlite,
    "DATE",
    ColumnType::Date,
    Value::null_date(),
    Value::date(chrono::NaiveDate::from_ymd_opt(1984, 1, 1).unwrap())
));

test_type!(datetime(
    sqlite,
    "DATETIME",
    ColumnType::DateTime,
    Value::null_datetime(),
    Value::datetime(chrono::DateTime::from_str("2020-07-29T09:23:44.458Z").unwrap())
));

#[quaint_test_macros::test_each_connector(tags("sqlite"))]
async fn test_type_text_datetime_rfc3339(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_type_table("DATETIME").await?;
    let dt = chrono::Utc::now();

    api.conn()
        .execute_raw(
            &format!("INSERT INTO {} (value) VALUES (?)", &table),
            &[Value::text(dt.to_rfc3339())],
        )
        .await?;

    let select = Select::from_table(&table).column("value").order_by("id".descend());
    let res = api.conn().select(select).await?.into_single()?;

    assert_eq!(Some(&Value::datetime(dt)), res.at(0));
    assert_matching_value_and_column_type(&res.types[0], res.at(0).unwrap());

    Ok(())
}

#[quaint_test_macros::test_each_connector(tags("sqlite"))]
async fn test_type_text_datetime_rfc2822(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_type_table("DATETIME").await?;
    let dt = chrono::DateTime::parse_from_rfc2822("Tue, 1 Jul 2003 10:52:37 +0200")
        .unwrap()
        .with_timezone(&chrono::Utc);

    api.conn()
        .execute_raw(
            &format!("INSERT INTO {} (value) VALUES (?)", &table),
            &[Value::text(dt.to_rfc2822())],
        )
        .await?;

    let select = Select::from_table(&table).column("value").order_by("id".descend());
    let res = api.conn().select(select).await?.into_single()?;

    assert_eq!(ColumnType::DateTime, res.types[0]);
    assert_eq!(Some(&Value::datetime(dt)), res.at(0));
    assert_matching_value_and_column_type(&res.types[0], res.at(0).unwrap());

    Ok(())
}

#[quaint_test_macros::test_each_connector(tags("sqlite"))]
async fn test_type_text_datetime_custom(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_type_table("DATETIME").await?;

    api.conn()
        .execute_raw(
            &format!("INSERT INTO {} (value) VALUES (?)", &table),
            &[Value::text("2020-04-20 16:20:00")],
        )
        .await?;

    let select = Select::from_table(&table).column("value").order_by("id".descend());
    let res = api.conn().select(select).await?.into_single()?;

    let naive = chrono::NaiveDateTime::parse_from_str("2020-04-20 16:20:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let expected = chrono::DateTime::from_naive_utc_and_offset(naive, chrono::Utc);

    assert_eq!(ColumnType::DateTime, res.types[0]);
    assert_eq!(Some(&Value::datetime(expected)), res.at(0));
    assert_matching_value_and_column_type(&res.types[0], res.at(0).unwrap());

    Ok(())
}

#[quaint_test_macros::test_each_connector(tags("sqlite"))]
async fn test_get_int64_from_int32_field_fails(api: &mut dyn TestApi) -> crate::Result<()> {
    let table = api.create_type_table("INT").await?;

    api.conn()
        .execute_raw(
            &format!("INSERT INTO {} (value) VALUES (9223372036854775807)", &table),
            &[],
        )
        .await?;

    let select = Select::from_table(&table).column("value").order_by("id".descend());
    let res = api.conn().select(select).await;

    assert!(res.is_err());

    Ok(())
}
