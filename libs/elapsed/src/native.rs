use std::time::{Duration, Instant};

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
