use derive_more::Display;

#[derive(Debug, Display)]
#[display("Failed to spawn a future")]
pub struct SpawnError;

impl std::error::Error for SpawnError {}
