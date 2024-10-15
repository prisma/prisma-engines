use derive_more::Display;

#[derive(Debug, Display)]
#[display(fmt = "Operation timed out")]
pub struct TimeoutError;

impl std::error::Error for TimeoutError {}
