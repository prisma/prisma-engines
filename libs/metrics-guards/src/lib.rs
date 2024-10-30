use std::sync::atomic::{AtomicBool, Ordering};

use metrics::gauge;

pub struct GaugeGuard {
    name: &'static str,
    decremented: AtomicBool,
}

impl GaugeGuard {
    pub fn increment(name: &'static str) -> Self {
        gauge!(name).increment(1.0);

        Self {
            name,
            decremented: AtomicBool::new(false),
        }
    }

    pub fn decrement(&self) {
        if !self.decremented.swap(true, Ordering::Relaxed) {
            gauge!(self.name).decrement(1.0);
        }
    }
}

impl Drop for GaugeGuard {
    fn drop(&mut self) {
        self.decrement();
    }
}
