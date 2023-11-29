use std::{
    future::Future,
    time::{Duration, Instant},
};

use crate::common::TimeoutError;

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
