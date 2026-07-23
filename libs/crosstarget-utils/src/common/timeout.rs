use derive_more::Display;

#[derive(Debug, Display)]
#[display("Operation timed out")]
pub struct TimeoutError;

impl std::error::Error for TimeoutError {}
