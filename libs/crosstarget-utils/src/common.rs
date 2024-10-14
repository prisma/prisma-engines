use std::fmt::Display;

#[derive(Debug)]
pub struct SpawnError;

impl Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to spawn a future")
    }
}

impl std::error::Error for SpawnError {}

#[derive(Debug)]
pub struct TimeoutError;

impl Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Operation timed out")
    }
}

impl std::error::Error for TimeoutError {}

#[derive(Debug)]
pub struct RegExpError {
    pub message: String,
}

impl Display for RegExpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Regular expression error: {}", self.message)
    }
}

impl std::error::Error for RegExpError {}

/// Test-relevant connector capabilities.
#[enumflags2::bitflags]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum RegExpFlags {
    IgnoreCase = 0b0001,
    Multiline = 0b0010,
}

impl RegExpFlags {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IgnoreCase => "i",
            Self::Multiline => "m",
        }
    }
}
