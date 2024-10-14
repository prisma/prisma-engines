use std::fmt::Display;

#[derive(Debug)]
pub struct SpawnError;

impl Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to spawn a future")
    }
}

impl std::error::Error for SpawnError {}
