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
    Utc.ymd(year, month, day)
        .and_hms_milli(or_zero(hour), or_zero(min), or_zero(sec), or_zero(millis))
        .to_rfc3339()
}

/// Convenience date string (UTC, RFC 3339) constructor so you don't have to remember the format.
pub fn date_iso_string(year: i32, month: u32, day: u32) -> String {
    Utc.ymd(year, month, day).and_hms_milli(0, 0, 0, 0).to_rfc3339()
}

/// Convenience date string (UTC, RFC 3339) constructor for the datetime right now,
/// for cases you don't care about the concrete DateTime in the tests.
pub fn now() -> String {
    Utc::now().to_rfc3339()
}

fn or_zero(opt: Option<u32>) -> u32 {
    opt.unwrap_or(0)
}
