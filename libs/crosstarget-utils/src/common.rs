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

#[derive(PartialEq)]
pub enum RegExpFlags {
    IgnoreCase,
    Multiline,
}

impl From<RegExpFlags> for String {
    fn from(flags: RegExpFlags) -> Self {
        match flags {
            RegExpFlags::IgnoreCase => "i",
            RegExpFlags::Multiline => "m",
        }
        .to_string()
    }
}
