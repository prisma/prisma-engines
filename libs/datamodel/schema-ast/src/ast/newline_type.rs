use std::fmt;

/// Defines the newline type of a PSL block.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum NewlineType {
    /// `\\n`
    #[default]
    Unix,
    /// `\\r\\n`
    Windows,
}

impl fmt::Display for NewlineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NewlineType::Unix => f.write_str("\n"),
            NewlineType::Windows => f.write_str("\r\n"),
        }
    }
}
