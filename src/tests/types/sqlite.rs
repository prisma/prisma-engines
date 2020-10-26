use crate::tests::test_api::sqlite_test_api;
#[cfg(feature = "chrono")]
use crate::tests::test_api::TestApi;
#[cfg(feature = "chrono")]
use crate::{ast::*, connector::Queryable};
use std::str::FromStr;

test_type!(integer(
    sqlite,
    "INTEGER",
    Value::Integer(None),
    Value::integer(i8::MIN),
    Value::integer(i8::MAX),
    Value::integer(i16::MIN),
    Value::integer(i16::MAX),
    Value::integer(i32::MIN),
    Value::integer(i32::MAX),
    Value::integer(i64::MIN),
    Value::integer(i64::MAX)
));

test_type!(real(
    sqlite,
    "REAL",
    Value::Real(None),
    Value::real(rust_decimal::Decimal::from_str("1.12345").unwrap())
));

test_type!(text(sqlite, "TEXT", Value::Text(None), Value::text("foobar huhuu")));

test_type!(blob(
    sqlite,
    "BLOB",
    Value::Bytes(None),
    Value::bytes(b"DEADBEEF".to_vec())
));

test_type!(boolean(
    sqlite,
    "BOOLEAN",
    Value::Boolean(None),
    Value::boolean(true),
    Value::boolean(false)
));

#[cfg(feature = "chrono")]
test_type!(date(
    sqlite,
    "DATE",
    Value::Date(None),
    Value::date(chrono::NaiveDate::from_ymd(1984, 1, 1))
));

#[cfg(feature = "chrono")]
test_type!(datetime(
    sqlite,
    "DATETIME",
    Value::DateTime(None),
    Value::datetime(chrono::DateTime::from_str("2020-07-29T09:23:44.458Z").unwrap())
));

#[cfg(feature = "chrono")]
#[test_macros::test_each_connector(tags("sqlite"))]
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

    Ok(())
}

#[cfg(feature = "chrono")]
#[test_macros::test_each_connector(tags("sqlite"))]
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

    assert_eq!(Some(&Value::datetime(dt)), res.at(0));

    Ok(())
}

#[cfg(feature = "chrono")]
#[test_macros::test_each_connector(tags("sqlite"))]
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
    let expected = chrono::DateTime::from_utc(naive, chrono::Utc);

    assert_eq!(Some(&Value::datetime(expected)), res.at(0));

    Ok(())
}
