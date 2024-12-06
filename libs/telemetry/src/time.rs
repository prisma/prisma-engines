use std::time::{Duration, SystemTime};

use serde::Serialize;

/// High-resolution time in the same format that OpenTelemetry uses.
///
/// The first number is Unix time in seconds since 00:00:00 UTC on 1 January 1970.
/// The second number is the sub-second amount of time elapsed since time represented by the first
/// number in nanoseconds.
///
/// ## Example
///
/// For example, `2021-01-01T12:30:10.150Z` in Unix time in milliseconds is 1609504210150.
/// Then the first number can be calculated by converting and truncating the epoch time in
/// milliseconds to seconds:
///
/// ```js
/// time[0] = Math.trunc(1609504210150 / 1000) // = 1609504210
/// ```
///
/// The second number can be calculated by converting the digits after the decimal point of the
/// expression `(1609504210150 / 1000) - time[0]` to nanoseconds:
///
/// ```js
/// time[1] = Number((1609504210.150 - time[0]).toFixed(9)) * 1e9 // = 150000000.
/// ```
///
/// Therefore, this time is represented in `HrTime` format as `[1609504210, 150000000]`.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct HrTime(u64, u32);

impl From<Duration> for HrTime {
    fn from(time: Duration) -> Self {
        Self(time.as_secs(), time.subsec_nanos())
    }
}

impl From<SystemTime> for HrTime {
    fn from(time: SystemTime) -> Self {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .expect("time can't be before unix epoch")
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_resolution_time_works() {
        // 2021-01-01T12:30:10.150Z in UNIX Epoch time in milliseconds
        let time_val = Duration::from_millis(1609504210150);
        assert_eq!(HrTime::from(time_val), HrTime(1609504210, 150000000));
    }
}
