use std::time::Duration;

/// TODO: this is a stub that always returns 0 as elapsed time
/// In should use performance::now() instead
pub struct ElapsedTimeCounter {}

impl ElapsedTimeCounter {
    pub fn start() -> Self {
        Self {}
    }

    pub fn elapsed_time(&self) -> Duration {
        Duration::from_millis(0u64)
    }
}
