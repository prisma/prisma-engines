use std::fmt::Display;

#[derive(Debug)]
pub struct SpawnError {}

impl SpawnError {
    pub fn new() -> Self {
        SpawnError {}
    }
}

impl Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to spawn a future")
    }
}

impl std::error::Error for SpawnError {}

#[derive(Debug)]
pub struct TimeoutError {}

impl TimeoutError {
    pub fn new() -> Self {
        TimeoutError {}
    }
}

impl Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Operation timed out")
    }
}

impl std::error::Error for TimeoutError {}
