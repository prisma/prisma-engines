use std::{
    future::Future,
    time::{Duration, Instant},
};

use crate::common::datetime::UtcDateTime;
use crate::common::timeout::TimeoutError;

pub use std::time::{SystemTime, SystemTimeError};

#[derive(Clone, Copy)]
pub struct ElapsedTimeCounter {
    instant: Instant,
}

impl ElapsedTimeCounter {
    pub fn start() -> Self {
        let instant = Instant::now();

        Self { instant }
    }

    pub fn elapsed_time(&self) -> Duration {
        self.instant.elapsed()
    }
}

pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await
}

pub async fn timeout<F>(duration: Duration, future: F) -> Result<F::Output, TimeoutError>
where
    F: Future + Send,
{
    let result = tokio::time::timeout(duration, future).await;

    result.map_err(|_| TimeoutError)
}

/// Native UTC DateTime implementation using chrono crate
#[derive(Clone, Debug)]
pub struct DateTime(chrono::DateTime<chrono::Utc>);

impl UtcDateTime for DateTime {
    fn now() -> Self {
        Self(chrono::Utc::now())
    }

    fn format(&self, format_str: &str) -> String {
        self.0.format(format_str).to_string()
    }
}

// Convenience function to get current timestamp formatted
pub fn format_utc_now(format_str: &str) -> String {
    DateTime::now().format(format_str)
}
