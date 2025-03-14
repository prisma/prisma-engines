// `clippy::empty_docs` is required because of the `wasm-bindgen` crate.
#![allow(clippy::empty_docs)]

use std::future::Future;
use std::time::Duration;

use derive_more::Display;
use js_sys::{Date, Function, Promise, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::common::datetime::UtcDateTime;
use crate::common::timeout::TimeoutError;

#[wasm_bindgen]
extern "C" {
    type Performance;

    #[wasm_bindgen(method)]
    fn now(this: &Performance) -> f64;

    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(closure: &Function, millis: u32) -> f64;

}

#[derive(Clone, Copy)]
pub struct ElapsedTimeCounter {
    start_time: f64,
}

impl ElapsedTimeCounter {
    pub fn start() -> Self {
        Self { start_time: now() }
    }

    pub fn elapsed_time(&self) -> Duration {
        Duration::from_millis((self.start_time - now()) as u64)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SystemTime(Duration);

impl SystemTime {
    pub const UNIX_EPOCH: Self = Self(Duration::ZERO);

    pub fn now() -> Self {
        let ms = js_sys::Date::now() as i64;
        let ms = ms.try_into().expect("negative timestamps are not supported");
        Self(Duration::from_millis(ms))
    }

    pub fn duration_since(&self, other: Self) -> Result<Duration, SystemTimeError> {
        self.0
            .checked_sub(other.0)
            .ok_or_else(|| SystemTimeError(other.0 - self.0))
    }
}

impl std::ops::Add<Duration> for SystemTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> SystemTime {
        Self(self.0 + rhs)
    }
}

#[derive(Clone, Debug, Display)]
#[display(fmt = "second time provided was later than self")]
pub struct SystemTimeError(Duration);

impl SystemTimeError {
    pub fn duration(&self) -> Duration {
        self.0
    }
}

impl std::error::Error for SystemTimeError {}

pub async fn sleep(duration: Duration) {
    let _ = JsFuture::from(Promise::new(&mut |resolve, _reject| {
        set_timeout(&resolve, duration.as_millis() as u32);
    }))
    .await;
}

pub async fn timeout<F>(duration: Duration, future: F) -> Result<F::Output, TimeoutError>
where
    F: Future,
{
    tokio::select! {
        result = future => Ok(result),
        _ = sleep(duration) => Err(TimeoutError)
    }
}

fn now() -> f64 {
    let global = js_sys::global();
    Reflect::get(&global, &"performance".into())
        .ok()
        .and_then(|value| {
            if value.is_undefined() {
                None
            } else {
                Some(Performance::from(value))
            }
        })
        .map(|p| p.now())
        .unwrap_or_else(Date::now)
}

#[derive(Clone, Debug)]
pub struct DateTime(js_sys::Date);

impl UtcDateTime for DateTime {
    fn now() -> Self {
        Self(js_sys::Date::new_0())
    }

    fn format(&self, format_str: &str) -> String {
        let mut chars = format_str.chars();
        let mut result = String::new();

        while let Some(c) = chars.next() {
            if c == '%' {
                if let Some(specifier) = chars.next() {
                    match specifier {
                        'Y' => result.push_str(&self.0.get_utc_full_year().to_string()),
                        'm' => result.push_str(&format!("{:02}", self.0.get_utc_month() + 1)), // JS months are 0-based
                        'd' => result.push_str(&format!("{:02}", self.0.get_utc_date())),
                        'H' => result.push_str(&format!("{:02}", self.0.get_utc_hours())),
                        'M' => result.push_str(&format!("{:02}", self.0.get_utc_minutes())),
                        'S' => result.push_str(&format!("{:02}", self.0.get_utc_seconds())),
                        _ => result.push_str(&format!("%{}", specifier)),
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }
}

// Convenience function to get current timestamp formatted
pub fn format_utc_now(format_str: &str) -> String {
    DateTime::now().format(format_str)
}
