use chrono::{TimeZone, Utc};

/// Convenience full date + time string (UTC, RFC 3339) constructor so you don't have to remember the format.
pub fn datetime_iso_string(
    year: i32,
    month: u32,
    day: u32,
    hour: Option<u32>,
    min: Option<u32>,
    sec: Option<u32>,
    millis: Option<u32>,
) -> String {
    Utc.with_ymd_and_hms(year, month, day, or_zero(hour), or_zero(min), or_zero(sec))
        .earliest()
        .and_then(|dt| dt.timezone().timestamp_millis_opt(or_zero(millis) as i64).earliest())
        .map(|dt| dt.to_rfc3339())
        .unwrap()
}

/// Convenience date string (UTC, RFC 3339) constructor so you don't have to remember the format.
pub fn date_iso_string(year: i32, month: u32, day: u32) -> String {
    Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
        .earliest()
        .map(|dt| dt.to_rfc3339())
        .unwrap()
}

/// Convenience date string (UTC, RFC 3339) constructor for the datetime right now,
/// for cases you don't care about the concrete DateTime in the tests.
pub fn now() -> String {
    Utc::now().to_rfc3339()
}

fn or_zero(opt: Option<u32>) -> u32 {
    opt.unwrap_or(0)
}
