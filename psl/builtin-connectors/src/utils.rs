use bigdecimal::{BigDecimal, ParseBigDecimalError};
use chrono::*;
use std::str::FromStr;

pub(crate) fn parse_date(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
    chrono::NaiveDate::parse_from_str(str, "%Y-%m-%d")
        .map(|date| DateTime::<Utc>::from_utc(date.and_hms_opt(0, 0, 0).unwrap(), Utc))
        .map(DateTime::<FixedOffset>::from)
}

pub(crate) fn parse_timestamptz(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(str)
}

pub(crate) fn parse_timestamp(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
    NaiveDateTime::parse_from_str(str, "%Y-%m-%dT%H:%M:%S%.f")
        .map(|dt| DateTime::from_utc(dt, Utc))
        .or_else(|_| DateTime::parse_from_rfc3339(str).map(DateTime::<Utc>::from))
        .map(DateTime::<FixedOffset>::from)
}

pub(crate) fn parse_time(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
    chrono::NaiveTime::parse_from_str(str, "%H:%M:%S%.f")
        .map(|time| {
            let base_date = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();

            DateTime::<Utc>::from_utc(base_date.and_time(time), Utc)
        })
        .map(DateTime::<FixedOffset>::from)
}

pub(crate) fn parse_timetz(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
    // We currently don't support time with timezone.
    // We strip the timezone information and parse it as a time.
    // This is inline with what Quaint does already.
    let time_without_tz = str.split('+').next().unwrap();

    parse_time(time_without_tz)
}

pub(crate) fn parse_money(str: &str) -> Result<BigDecimal, ParseBigDecimalError> {
    // We strip out the currency sign from the string.
    BigDecimal::from_str(&str[1..]).map(|bd| bd.normalized())
}

pub(crate) fn parse_decimal(str: &str) -> Result<BigDecimal, ParseBigDecimalError> {
    BigDecimal::from_str(str).map(|bd| bd.normalized())
}
