#[cfg(any(feature = "postgresql", feature = "mysql"))]
pub(crate) mod common {
    use chrono::*;

    pub(crate) fn parse_date(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
        chrono::NaiveDate::parse_from_str(str, "%Y-%m-%d")
            .map(|date| DateTime::<Utc>::from_naive_utc_and_offset(date.and_hms_opt(0, 0, 0).unwrap(), Utc))
            .map(DateTime::<FixedOffset>::from)
    }

    pub(crate) fn parse_time(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
        chrono::NaiveTime::parse_from_str(str, "%H:%M:%S%.f")
            .map(|time| {
                let base_date = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();

                DateTime::<Utc>::from_naive_utc_and_offset(base_date.and_time(time), Utc)
            })
            .map(DateTime::<FixedOffset>::from)
    }

    pub(crate) fn parse_timestamp(str: &str, fmt: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
        NaiveDateTime::parse_from_str(str, fmt)
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
            .or_else(|_| DateTime::parse_from_rfc3339(str).map(DateTime::<Utc>::from))
            .map(DateTime::<FixedOffset>::from)
    }
}

#[cfg(feature = "postgresql")]
pub(crate) mod postgres {
    use alloc::vec::Vec;
    use chrono::*;

    pub(crate) fn parse_timestamptz(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
        DateTime::parse_from_rfc3339(str)
    }

    pub(crate) fn parse_timestamp(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
        super::common::parse_timestamp(str, "%Y-%m-%dT%H:%M:%S%.f")
    }

    pub(crate) fn parse_timetz(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
        // We currently don't support time with timezone.
        // We strip the timezone information and parse it as a time.
        // This is inline with what Quaint does already.
        let time_without_tz = str.split('+').next().unwrap();

        super::common::parse_time(time_without_tz)
    }

    pub(crate) fn parse_bytes(str: &str) -> Result<Vec<u8>, hex::FromHexError> {
        hex::decode(&str[2..])
    }
}

#[cfg(feature = "mysql")]
pub(crate) mod mysql {
    use chrono::*;

    pub(crate) fn parse_datetime(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
        super::common::parse_timestamp(str, "%Y-%m-%d %H:%M:%S%.f")
    }

    pub(crate) fn parse_timestamp(str: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
        parse_datetime(str)
    }
}
